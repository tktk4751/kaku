//! SQLiteベースのノートインデックス
//!
//! # 概要
//!
//! 10万件以上のノートでも高速に動作するよう、メタデータをSQLiteで管理します。
//! ファイルシステムの全スキャンを回避し、O(1)〜O(log n)でのアクセスを実現。
//!
//! # 機能
//!
//! - ノートメタデータの永続化
//! - FTS5による全文検索
//! - バックリンク管理
//! - タイトル→UID逆引き
//!
//! # スレッドセーフティ
//!
//! Connection は Mutex で保護されており、複数スレッドから安全にアクセス可能。

use crate::domain::backlink::extract_wiki_links;
use crate::traits::NoteListItem;
use chrono::{DateTime, NaiveDateTime, Utc};
use parking_lot::Mutex;
use rusqlite::{params, Connection};
use std::path::{Path, PathBuf};
use thiserror::Error;

/// SQLiteインデックスのエラー型
#[derive(Debug, Error)]
pub enum IndexError {
    #[error("SQLiteエラー: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("I/Oエラー: {0}")]
    Io(#[from] std::io::Error),

    #[error("データ不整合: {0}")]
    DataInconsistency(String),
}

/// インデックス用ノート情報
#[derive(Debug, Clone)]
pub struct IndexedNote {
    pub uid: String,
    pub title: String,
    pub content: String,
    pub file_path: PathBuf,
    pub content_hash: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// ギャラリー用ノート情報（プレビュー・タグ付き）
#[derive(Debug, Clone)]
pub struct GalleryNote {
    pub uid: String,
    pub title: String,
    pub preview: String,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// バックリンク情報（SQLite用）
#[derive(Debug, Clone)]
pub struct IndexedBacklink {
    pub source_uid: String,
    pub source_title: String,
}

/// SQLiteインデックスマネージャー
pub struct SqliteIndex {
    conn: Mutex<Connection>,
    #[allow(dead_code)]
    db_path: PathBuf,
}

impl SqliteIndex {
    /// 新規作成または既存DBを開く
    pub fn open(db_path: PathBuf) -> Result<Self, IndexError> {
        // 親ディレクトリを作成
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(&db_path)?;

        // パフォーマンス設定
        conn.execute_batch(
            "
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;
            PRAGMA cache_size = -64000;
            PRAGMA temp_store = MEMORY;
            ",
        )?;

        let index = Self {
            conn: Mutex::new(conn),
            db_path,
        };

        index.run_migrations()?;
        Ok(index)
    }

    /// インメモリDBで作成（テスト用）
    #[cfg(test)]
    pub fn open_in_memory() -> Result<Self, IndexError> {
        let conn = Connection::open_in_memory()?;
        let index = Self {
            conn: Mutex::new(conn),
            db_path: PathBuf::from(":memory:"),
        };
        index.run_migrations()?;
        Ok(index)
    }

    /// スキーママイグレーション実行
    fn run_migrations(&self) -> Result<(), IndexError> {
        let conn = self.conn.lock();

        // スキーマバージョンテーブルを作成（存在しなければ）
        conn.execute(
            "CREATE TABLE IF NOT EXISTS schema_version (
                version INTEGER PRIMARY KEY,
                applied_at TEXT NOT NULL
            )",
            [],
        )?;

        // 現在のバージョン確認
        let current_version: i32 = conn
            .query_row(
                "SELECT COALESCE(MAX(version), 0) FROM schema_version",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

        // マイグレーション V1: 基本テーブル
        if current_version < 1 {
            conn.execute_batch(
                "
                -- メインインデックステーブル
                CREATE TABLE IF NOT EXISTS notes (
                    uid TEXT PRIMARY KEY,
                    title TEXT NOT NULL,
                    file_path TEXT NOT NULL UNIQUE,
                    content_hash TEXT NOT NULL,
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL,
                    indexed_at TEXT NOT NULL
                );

                -- 高速検索用インデックス
                CREATE INDEX IF NOT EXISTS idx_notes_updated_at ON notes(updated_at DESC);
                CREATE INDEX IF NOT EXISTS idx_notes_title ON notes(title);

                -- FTS5全文検索テーブル
                CREATE VIRTUAL TABLE IF NOT EXISTS notes_fts USING fts5(
                    uid UNINDEXED,
                    title,
                    content,
                    tokenize='unicode61'
                );

                -- バックリンクテーブル
                CREATE TABLE IF NOT EXISTS backlinks (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    source_uid TEXT NOT NULL,
                    target_title TEXT NOT NULL,
                    position INTEGER NOT NULL,
                    FOREIGN KEY (source_uid) REFERENCES notes(uid) ON DELETE CASCADE
                );

                CREATE INDEX IF NOT EXISTS idx_backlinks_target ON backlinks(target_title);
                CREATE INDEX IF NOT EXISTS idx_backlinks_source ON backlinks(source_uid);

                -- タイトル→UID逆引きテーブル（O(1)検索用）
                CREATE TABLE IF NOT EXISTS title_index (
                    title_normalized TEXT PRIMARY KEY,
                    uid TEXT NOT NULL,
                    FOREIGN KEY (uid) REFERENCES notes(uid) ON DELETE CASCADE
                );

                INSERT INTO schema_version (version, applied_at) VALUES (1, datetime('now'));
                ",
            )?;
        }

        // マイグレーション V2: ギャラリー用カラム追加
        if current_version < 2 {
            conn.execute_batch(
                "
                -- プレビューとタグカラムを追加
                ALTER TABLE notes ADD COLUMN preview TEXT NOT NULL DEFAULT '';
                ALTER TABLE notes ADD COLUMN tags_json TEXT NOT NULL DEFAULT '[]';

                -- ギャラリー用インデックス
                CREATE INDEX IF NOT EXISTS idx_notes_created_at ON notes(created_at DESC);

                INSERT INTO schema_version (version, applied_at) VALUES (2, datetime('now'));
                ",
            )?;
        }

        Ok(())
    }

    /// ノートをインデックスに追加/更新
    pub fn upsert_note(&self, note: &IndexedNote) -> Result<(), IndexError> {
        self.upsert_note_with_gallery(note, "", &[])
    }

    /// ノートをインデックスに追加/更新（ギャラリー情報付き）
    pub fn upsert_note_with_gallery(
        &self,
        note: &IndexedNote,
        preview: &str,
        tags: &[String],
    ) -> Result<(), IndexError> {
        let conn = self.conn.lock();
        let now = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let tags_json = serde_json::to_string(tags).unwrap_or_else(|_| "[]".to_string());

        conn.execute(
            "INSERT INTO notes (uid, title, file_path, content_hash, created_at, updated_at, indexed_at, preview, tags_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT(uid) DO UPDATE SET
                title = excluded.title,
                file_path = excluded.file_path,
                content_hash = excluded.content_hash,
                updated_at = excluded.updated_at,
                indexed_at = ?7,
                preview = ?8,
                tags_json = ?9",
            params![
                note.uid,
                note.title,
                note.file_path.to_string_lossy().to_string(),
                note.content_hash,
                format_datetime(&note.created_at),
                format_datetime(&note.updated_at),
                now,
                preview,
                tags_json,
            ],
        )?;

        // FTS更新
        conn.execute("DELETE FROM notes_fts WHERE uid = ?1", params![note.uid])?;
        conn.execute(
            "INSERT INTO notes_fts (uid, title, content) VALUES (?1, ?2, ?3)",
            params![note.uid, note.title, note.content],
        )?;

        // バックリンク更新
        self.update_backlinks_internal(&conn, &note.uid, &note.content)?;

        // タイトルインデックス更新
        let title_normalized = note.title.to_lowercase();
        // 古いタイトルを削除してから新しいタイトルを挿入
        conn.execute(
            "DELETE FROM title_index WHERE uid = ?1",
            params![note.uid],
        )?;
        conn.execute(
            "INSERT OR REPLACE INTO title_index (title_normalized, uid) VALUES (?1, ?2)",
            params![title_normalized, note.uid],
        )?;

        Ok(())
    }

    /// ノートをインデックスから削除
    pub fn delete_note(&self, uid: &str) -> Result<(), IndexError> {
        let conn = self.conn.lock();

        // タイトルインデックスを削除
        conn.execute("DELETE FROM title_index WHERE uid = ?1", params![uid])?;

        // バックリンクを削除
        conn.execute("DELETE FROM backlinks WHERE source_uid = ?1", params![uid])?;

        // FTSを削除
        conn.execute("DELETE FROM notes_fts WHERE uid = ?1", params![uid])?;

        // メインテーブルを削除
        conn.execute("DELETE FROM notes WHERE uid = ?1", params![uid])?;

        Ok(())
    }

    /// ノート一覧を取得（ページネーション対応）
    pub fn list_notes(
        &self,
        offset: usize,
        limit: usize,
    ) -> Result<(Vec<NoteListItem>, usize), IndexError> {
        let conn = self.conn.lock();

        let total: usize =
            conn.query_row("SELECT COUNT(*) FROM notes", [], |row| row.get(0))?;

        let mut stmt = conn.prepare(
            "SELECT uid, title, file_path, updated_at
             FROM notes
             ORDER BY updated_at DESC
             LIMIT ?1 OFFSET ?2",
        )?;

        let items: Vec<NoteListItem> = stmt
            .query_map(params![limit as i64, offset as i64], |row| {
                let uid: String = row.get(0)?;
                let title: String = row.get(1)?;
                let file_path: String = row.get(2)?;
                let updated_at_str: String = row.get(3)?;
                let updated_at = parse_datetime(&updated_at_str);

                Ok(NoteListItem {
                    uid,
                    title,
                    path: PathBuf::from(file_path),
                    updated_at,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok((items, total))
    }

    /// 全件取得（後方互換用）
    pub fn list_all_notes(&self) -> Result<Vec<NoteListItem>, IndexError> {
        let (items, _) = self.list_notes(0, usize::MAX)?;
        Ok(items)
    }

    /// ギャラリー用ノート一覧を取得（キャッシュから高速取得）
    pub fn list_gallery_notes(
        &self,
        sort_by_created: bool,
        tag_filter: Option<&str>,
    ) -> Result<Vec<GalleryNote>, IndexError> {
        let conn = self.conn.lock();

        let order = if sort_by_created {
            "created_at DESC"
        } else {
            "updated_at DESC"
        };

        let query = format!(
            "SELECT uid, title, preview, tags_json, created_at, updated_at
             FROM notes
             ORDER BY {}",
            order
        );

        let mut stmt = conn.prepare(&query)?;

        let items: Vec<GalleryNote> = stmt
            .query_map([], |row| {
                let uid: String = row.get(0)?;
                let title: String = row.get(1)?;
                let preview: String = row.get(2)?;
                let tags_json: String = row.get(3)?;
                let created_at_str: String = row.get(4)?;
                let updated_at_str: String = row.get(5)?;

                let tags: Vec<String> =
                    serde_json::from_str(&tags_json).unwrap_or_default();

                Ok(GalleryNote {
                    uid,
                    title,
                    preview,
                    tags,
                    created_at: parse_datetime(&created_at_str),
                    updated_at: parse_datetime(&updated_at_str),
                })
            })?
            .filter_map(|r| r.ok())
            .filter(|note| {
                // タグフィルタを適用
                if let Some(filter) = tag_filter {
                    note.tags.iter().any(|t| t == filter)
                } else {
                    true
                }
            })
            .collect();

        Ok(items)
    }

    /// UIDからファイルパスを取得
    pub fn get_path(&self, uid: &str) -> Result<Option<PathBuf>, IndexError> {
        let conn = self.conn.lock();

        let result = conn.query_row(
            "SELECT file_path FROM notes WHERE uid = ?1",
            params![uid],
            |row| {
                let path: String = row.get(0)?;
                Ok(PathBuf::from(path))
            },
        );

        match result {
            Ok(path) => Ok(Some(path)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// タイトルでノートを検索（O(1)）
    pub fn find_by_title(&self, title: &str) -> Result<Option<String>, IndexError> {
        let conn = self.conn.lock();
        let title_normalized = title.to_lowercase();

        let result = conn.query_row(
            "SELECT uid FROM title_index WHERE title_normalized = ?1",
            params![title_normalized],
            |row| row.get(0),
        );

        match result {
            Ok(uid) => Ok(Some(uid)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// UIDからNoteListItemを取得（O(1)）
    pub fn get_note_by_uid(&self, uid: &str) -> Result<Option<NoteListItem>, IndexError> {
        let conn = self.conn.lock();

        let result = conn.query_row(
            "SELECT uid, title, file_path, updated_at FROM notes WHERE uid = ?1",
            params![uid],
            |row| {
                let uid: String = row.get(0)?;
                let title: String = row.get(1)?;
                let file_path: String = row.get(2)?;
                let updated_at_str: String = row.get(3)?;
                let updated_at = parse_datetime(&updated_at_str);

                Ok(NoteListItem {
                    uid,
                    title,
                    path: PathBuf::from(file_path),
                    updated_at,
                })
            },
        );

        match result {
            Ok(item) => Ok(Some(item)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// バックリンク取得
    pub fn get_backlinks(&self, uid: &str) -> Result<Vec<IndexedBacklink>, IndexError> {
        let conn = self.conn.lock();

        // まず対象ノートのタイトルを取得
        let title: String = match conn.query_row(
            "SELECT title FROM notes WHERE uid = ?1",
            params![uid],
            |row| row.get(0),
        ) {
            Ok(t) => t,
            Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(Vec::new()),
            Err(e) => return Err(e.into()),
        };

        let title_normalized = title.to_lowercase();

        // そのタイトルへのリンクを持つノートを検索
        let mut stmt = conn.prepare(
            "SELECT DISTINCT n.uid, n.title
             FROM backlinks b
             JOIN notes n ON b.source_uid = n.uid
             WHERE b.target_title = ?1",
        )?;

        let backlinks: Vec<IndexedBacklink> = stmt
            .query_map(params![title_normalized], |row| {
                Ok(IndexedBacklink {
                    source_uid: row.get(0)?,
                    source_title: row.get(1)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(backlinks)
    }

    /// コンテンツハッシュで更新が必要か判定
    pub fn needs_update(&self, uid: &str, content_hash: &str) -> Result<bool, IndexError> {
        let conn = self.conn.lock();

        let result = conn.query_row(
            "SELECT content_hash FROM notes WHERE uid = ?1",
            params![uid],
            |row| {
                let hash: String = row.get(0)?;
                Ok(hash)
            },
        );

        match result {
            Ok(existing_hash) => Ok(existing_hash != content_hash),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(true), // 存在しない = 更新必要
            Err(e) => Err(e.into()),
        }
    }

    /// 孤立したインデックスエントリを削除
    ///
    /// ファイルが存在しないエントリを削除し、削除数を返す
    pub fn remove_orphans(&self, base_dir: &Path) -> Result<usize, IndexError> {
        let conn = self.conn.lock();

        // 全エントリを取得
        let mut stmt = conn.prepare("SELECT uid, file_path FROM notes")?;
        let entries: Vec<(String, PathBuf)> = stmt
            .query_map([], |row| {
                let uid: String = row.get(0)?;
                let path: String = row.get(1)?;
                Ok((uid, PathBuf::from(path)))
            })?
            .filter_map(|r| r.ok())
            .collect();

        let mut removed = 0;

        for (uid, path) in entries {
            // base_dirを考慮してパスが存在するか確認
            let full_path = if path.is_absolute() {
                path
            } else {
                base_dir.join(&path)
            };

            if !full_path.exists() {
                // 削除（lockを解放してからは呼べないので、直接SQLを実行）
                conn.execute("DELETE FROM title_index WHERE uid = ?1", params![uid])?;
                conn.execute("DELETE FROM backlinks WHERE source_uid = ?1", params![uid])?;
                conn.execute("DELETE FROM notes_fts WHERE uid = ?1", params![uid])?;
                conn.execute("DELETE FROM notes WHERE uid = ?1", params![uid])?;
                removed += 1;
            }
        }

        Ok(removed)
    }

    /// インデックスの完全再構築
    pub fn rebuild_full<I>(&self, notes: I) -> Result<(), IndexError>
    where
        I: Iterator<Item = IndexedNote>,
    {
        let conn = self.conn.lock();

        // トランザクション開始
        conn.execute("BEGIN TRANSACTION", [])?;

        // 全テーブルクリア
        conn.execute("DELETE FROM title_index", [])?;
        conn.execute("DELETE FROM backlinks", [])?;
        conn.execute("DELETE FROM notes_fts", [])?;
        conn.execute("DELETE FROM notes", [])?;

        let now = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

        // バルクインサート
        for note in notes {
            conn.execute(
                "INSERT INTO notes (uid, title, file_path, content_hash, created_at, updated_at, indexed_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    note.uid,
                    note.title,
                    note.file_path.to_string_lossy().to_string(),
                    note.content_hash,
                    format_datetime(&note.created_at),
                    format_datetime(&note.updated_at),
                    now,
                ],
            )?;

            // FTS
            conn.execute(
                "INSERT INTO notes_fts (uid, title, content) VALUES (?1, ?2, ?3)",
                params![note.uid, note.title, note.content],
            )?;

            // バックリンク
            self.update_backlinks_internal(&conn, &note.uid, &note.content)?;

            // タイトルインデックス
            let title_normalized = note.title.to_lowercase();
            conn.execute(
                "INSERT OR REPLACE INTO title_index (title_normalized, uid) VALUES (?1, ?2)",
                params![title_normalized, note.uid],
            )?;
        }

        conn.execute("COMMIT", [])?;

        Ok(())
    }

    /// 再構築が必要か判定（DBが空の場合）
    pub fn needs_rebuild(&self) -> Result<bool, IndexError> {
        let conn = self.conn.lock();
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM notes", [], |row| row.get(0))?;
        Ok(count == 0)
    }

    /// バックリンクの内部更新
    fn update_backlinks_internal(
        &self,
        conn: &Connection,
        uid: &str,
        content: &str,
    ) -> Result<(), IndexError> {
        // 既存のバックリンクを削除
        conn.execute("DELETE FROM backlinks WHERE source_uid = ?1", params![uid])?;

        // WikiLinkを抽出して挿入
        let links = extract_wiki_links(content);
        for link in links {
            let target_normalized = link.title.to_lowercase();
            conn.execute(
                "INSERT INTO backlinks (source_uid, target_title, position) VALUES (?1, ?2, ?3)",
                params![uid, target_normalized, link.position as i64],
            )?;
        }

        Ok(())
    }

    /// インデックスのノート数を取得
    pub fn count(&self) -> Result<usize, IndexError> {
        let conn = self.conn.lock();
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM notes", [], |row| row.get(0))?;
        Ok(count as usize)
    }
}

/// 日時をフォーマット
fn format_datetime(dt: &DateTime<Utc>) -> String {
    dt.format("%Y-%m-%d %H:%M:%S").to_string()
}

/// 日時をパース
fn parse_datetime(s: &str) -> DateTime<Utc> {
    NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
        .map(|naive| naive.and_utc())
        .unwrap_or_else(|_| Utc::now())
}

/// コンテンツハッシュを計算
pub fn compute_hash(content: &str) -> String {
    let hash = blake3::hash(content.as_bytes());
    hash.to_hex().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_note(uid: &str, title: &str, content: &str) -> IndexedNote {
        IndexedNote {
            uid: uid.to_string(),
            title: title.to_string(),
            content: content.to_string(),
            file_path: PathBuf::from(format!("/test/{}.md", uid)),
            content_hash: compute_hash(content),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn test_upsert_and_list() {
        let index = SqliteIndex::open_in_memory().unwrap();

        let note1 = create_test_note("001", "Test Note 1", "# Test Note 1\n\nContent");
        let note2 = create_test_note("002", "Test Note 2", "# Test Note 2\n\nMore content");

        index.upsert_note(&note1).unwrap();
        index.upsert_note(&note2).unwrap();

        let (items, total) = index.list_notes(0, 10).unwrap();
        assert_eq!(total, 2);
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn test_find_by_title() {
        let index = SqliteIndex::open_in_memory().unwrap();

        let note = create_test_note("001", "My Unique Title", "Content");
        index.upsert_note(&note).unwrap();

        // 正確なタイトルで検索
        let found = index.find_by_title("My Unique Title").unwrap();
        assert_eq!(found, Some("001".to_string()));

        // 大文字小文字を無視
        let found = index.find_by_title("my unique title").unwrap();
        assert_eq!(found, Some("001".to_string()));

        // 存在しないタイトル
        let found = index.find_by_title("Nonexistent").unwrap();
        assert!(found.is_none());
    }

    #[test]
    fn test_backlinks() {
        let index = SqliteIndex::open_in_memory().unwrap();

        let note1 = create_test_note("001", "Target Note", "# Target Note\n\nThis is the target.");
        let note2 = create_test_note(
            "002",
            "Source Note",
            "# Source Note\n\nLink to [[Target Note]].",
        );

        index.upsert_note(&note1).unwrap();
        index.upsert_note(&note2).unwrap();

        let backlinks = index.get_backlinks("001").unwrap();
        assert_eq!(backlinks.len(), 1);
        assert_eq!(backlinks[0].source_uid, "002");
        assert_eq!(backlinks[0].source_title, "Source Note");
    }

    #[test]
    fn test_delete_note() {
        let index = SqliteIndex::open_in_memory().unwrap();

        let note = create_test_note("001", "To Delete", "Content");
        index.upsert_note(&note).unwrap();

        assert_eq!(index.count().unwrap(), 1);

        index.delete_note("001").unwrap();
        assert_eq!(index.count().unwrap(), 0);
    }

    #[test]
    fn test_needs_update() {
        let index = SqliteIndex::open_in_memory().unwrap();

        let note = create_test_note("001", "Test", "Content");
        index.upsert_note(&note).unwrap();

        // 同じハッシュ → 更新不要
        assert!(!index.needs_update("001", &note.content_hash).unwrap());

        // 違うハッシュ → 更新必要
        assert!(index.needs_update("001", "different_hash").unwrap());

        // 存在しないUID → 更新必要（新規）
        assert!(index.needs_update("999", "any_hash").unwrap());
    }

    #[test]
    fn test_pagination() {
        let index = SqliteIndex::open_in_memory().unwrap();

        // 10件のノートを作成
        for i in 0..10 {
            let note = create_test_note(
                &format!("{:03}", i),
                &format!("Note {}", i),
                &format!("Content {}", i),
            );
            index.upsert_note(&note).unwrap();
        }

        // 最初の3件
        let (items, total) = index.list_notes(0, 3).unwrap();
        assert_eq!(total, 10);
        assert_eq!(items.len(), 3);

        // 次の3件
        let (items, _) = index.list_notes(3, 3).unwrap();
        assert_eq!(items.len(), 3);

        // 最後の件
        let (items, _) = index.list_notes(9, 10).unwrap();
        assert_eq!(items.len(), 1);
    }
}
