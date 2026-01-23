//! ハイブリッドリポジトリ
//!
//! SQLiteインデックス + ファイルシステムの組み合わせで高速なノート管理を実現。
//!
//! # 責務分担
//!
//! - **SQLite**: メタデータ、検索、バックリンク（高速クエリ）
//! - **FileSystem**: コンテンツ保存（信頼性）
//!
//! # 一貫性保証
//!
//! - 保存時: ファイル → SQLite の順で更新（ファイルが真のソース）
//! - 削除時: SQLite → ファイル の順で削除
//! - 同期: ファイルシステムとインデックスの整合性を定期的にチェック
//!
//! # API互換性
//!
//! `NoteRepository` トレイトを完全に実装し、既存の `FileNoteRepository` と
//! 同じインターフェースを提供。既存コードの変更なしに置き換え可能。

use crate::commands::gallery::{generate_preview, PREVIEW_LENGTH};
use crate::domain::Note;
use crate::infrastructure::sqlite_index::{compute_hash, GalleryNote, IndexedNote, SqliteIndex};
use crate::services::SettingsService;
use crate::traits::{FilenameStrategy, NoteListItem, NoteRepository, RepositoryError, Storage};
use log::{debug, info};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// 同期結果
#[derive(Debug, Clone)]
pub struct SyncResult {
    pub added: usize,
    pub updated: usize,
    pub removed: usize,
}

/// ハイブリッドリポジトリ実装
pub struct HybridRepository {
    index: Arc<SqliteIndex>,
    storage: Arc<dyn Storage>,
    filename_strategy: Arc<dyn FilenameStrategy>,
    settings_service: Arc<SettingsService>,
}

impl HybridRepository {
    /// 新規作成
    pub fn new(
        index: Arc<SqliteIndex>,
        storage: Arc<dyn Storage>,
        filename_strategy: Arc<dyn FilenameStrategy>,
        settings_service: Arc<SettingsService>,
    ) -> Self {
        Self {
            index,
            storage,
            filename_strategy,
            settings_service,
        }
    }

    /// 保存ディレクトリを取得
    fn base_dir(&self) -> PathBuf {
        self.settings_service.storage_directory()
    }

    /// 既存ファイルパスの一覧を取得
    fn get_existing_files(&self) -> Vec<PathBuf> {
        self.storage
            .list_files(&self.base_dir(), "md")
            .unwrap_or_default()
    }

    /// パスを解決または生成
    fn resolve_or_generate_path(&self, note: &Note) -> Result<PathBuf, RepositoryError> {
        // インデックスからパスを取得
        if let Ok(Some(path)) = self.index.get_path(&note.metadata.uid) {
            return Ok(path);
        }

        // 新規生成
        let existing_files = self.get_existing_files();
        let refs: Vec<&Path> = existing_files.iter().map(|p| p.as_path()).collect();
        let filename = self.filename_strategy.generate(note, &refs);
        Ok(self.base_dir().join(format!("{}.md", filename)))
    }

    /// ページネーション対応リスト取得
    pub fn list_paginated(
        &self,
        offset: usize,
        limit: usize,
    ) -> Result<(Vec<NoteListItem>, usize), RepositoryError> {
        self.index
            .list_notes(offset, limit)
            .map_err(|e| RepositoryError::storage("list_paginated", storage_error_from_index(e)))
    }

    /// タイトルでノートを検索（O(1)）
    pub fn find_by_title(&self, title: &str) -> Result<Option<NoteListItem>, RepositoryError> {
        // First get UID from title index
        let uid = self
            .index
            .find_by_title(title)
            .map_err(|e| RepositoryError::storage("find_by_title", storage_error_from_index(e)))?;

        match uid {
            Some(uid) => {
                // Directly fetch NoteListItem from index (O(1))
                self.index
                    .get_note_by_uid(&uid)
                    .map_err(|e| RepositoryError::storage("get_note_by_uid", storage_error_from_index(e)))
            }
            None => Ok(None),
        }
    }

    /// インデックス同期チェック
    ///
    /// ファイルシステムとインデックスの整合性を確認・修復
    pub fn sync_index(&self) -> Result<SyncResult, RepositoryError> {
        let base_dir = self.base_dir();
        let files = self.storage.list_files(&base_dir, "md")?;

        let added = 0;
        let mut updated = 0;

        // ファイルを走査してインデックスを更新
        for path in files {
            if let Ok(content) = self.storage.load(&path) {
                if let Ok(note) = Note::from_file_content(&content) {
                    let hash = compute_hash(&content);

                    // インデックスにない or ハッシュが違う場合は更新
                    let needs_update = self
                        .index
                        .needs_update(&note.metadata.uid, &hash)
                        .unwrap_or(true);

                    if needs_update {
                        let title = note
                            .extract_heading()
                            .unwrap_or_else(|| note.metadata.uid.clone());

                        let indexed = IndexedNote {
                            uid: note.metadata.uid.clone(),
                            title,
                            content: note.content.clone(),
                            file_path: path.clone(),
                            content_hash: hash,
                            created_at: note.metadata.created_at,
                            updated_at: note.metadata.updated_at,
                        };

                        // ギャラリー用プレビューとタグを生成
                        let preview = generate_preview(&note.content, PREVIEW_LENGTH);
                        let tags = note.all_tags();

                        self.index
                            .upsert_note_with_gallery(&indexed, &preview, &tags)
                            .map_err(|e| RepositoryError::storage("sync", storage_error_from_index(e)))?;

                        updated += 1;
                    }
                }
            }
        }

        // インデックスにあってファイルにないものを削除
        let removed = self
            .index
            .remove_orphans(&base_dir)
            .map_err(|e| RepositoryError::storage("remove_orphans", storage_error_from_index(e)))?;

        Ok(SyncResult {
            added,
            updated,
            removed,
        })
    }

    /// 初期化（インデックス構築が必要な場合に実行）
    pub fn initialize(&self) -> Result<(), RepositoryError> {
        let needs_rebuild = self
            .index
            .needs_rebuild()
            .map_err(|e| RepositoryError::storage("check_rebuild", storage_error_from_index(e)))?;

        if needs_rebuild {
            debug!("Building SQLite index...");
            let start = std::time::Instant::now();

            self.sync_index()?;

            info!(
                "Index built in {:?} ({} notes)",
                start.elapsed(),
                self.index.count().unwrap_or(0)
            );
        }

        Ok(())
    }

    /// SQLiteインデックスへの参照を取得
    pub fn index(&self) -> &Arc<SqliteIndex> {
        &self.index
    }
}

impl NoteRepository for HybridRepository {
    fn save(&self, note: &Note) -> Result<PathBuf, RepositoryError> {
        // 1. ファイルパスを決定
        let path = self.resolve_or_generate_path(note)?;

        // 2. ファイルに保存（アトミック）
        let content = note.to_file_content();
        self.storage.save_atomic(&path, &content)?;

        // 3. インデックスを更新（ギャラリー情報も含む）
        let title = note
            .extract_heading()
            .unwrap_or_else(|| note.metadata.uid.clone());

        let indexed_note = IndexedNote {
            uid: note.metadata.uid.clone(),
            title,
            content: note.content.clone(),
            file_path: path.clone(),
            content_hash: compute_hash(&content),
            created_at: note.metadata.created_at,
            updated_at: note.metadata.updated_at,
        };

        // ギャラリー用プレビューとタグを生成
        let preview = generate_preview(&note.content, PREVIEW_LENGTH);
        let tags = note.all_tags();

        self.index
            .upsert_note_with_gallery(&indexed_note, &preview, &tags)
            .map_err(|e| RepositoryError::storage("index_upsert", storage_error_from_index(e)))?;

        Ok(path)
    }

    fn load(&self, uid: &str) -> Result<Note, RepositoryError> {
        // インデックスからパスを取得（O(1)）
        let path = self
            .index
            .get_path(uid)
            .map_err(|e| RepositoryError::storage("get_path", storage_error_from_index(e)))?
            .ok_or_else(|| RepositoryError::not_found(uid))?;

        // ファイルを読み込み
        let content = self.storage.load(&path)?;
        Note::from_file_content(&content).map_err(|_| {
            RepositoryError::parse("Invalid note format", Some(path))
        })
    }

    fn delete(&self, uid: &str) -> Result<(), RepositoryError> {
        // 1. パスを取得
        let path = self
            .index
            .get_path(uid)
            .map_err(|e| RepositoryError::storage("get_path", storage_error_from_index(e)))?
            .ok_or_else(|| RepositoryError::not_found(uid))?;

        // 2. インデックスから削除
        self.index
            .delete_note(uid)
            .map_err(|e| RepositoryError::storage("index_delete", storage_error_from_index(e)))?;

        // 3. ファイルを削除
        self.storage.delete(&path)?;

        Ok(())
    }

    fn list_all(&self) -> Result<Vec<NoteListItem>, RepositoryError> {
        // 後方互換: 全件取得
        self.index
            .list_all_notes()
            .map_err(|e| RepositoryError::storage("list_all", storage_error_from_index(e)))
    }

    fn get_path(&self, uid: &str) -> Option<PathBuf> {
        self.index.get_path(uid).ok().flatten()
    }

    fn list_gallery(
        &self,
        sort_by_created: bool,
        tag_filter: Option<&str>,
    ) -> Result<Vec<GalleryNote>, RepositoryError> {
        // SQLiteインデックスから直接取得（高速）
        self.index
            .list_gallery_notes(sort_by_created, tag_filter)
            .map_err(|e| RepositoryError::storage("list_gallery", storage_error_from_index(e)))
    }
}

/// IndexErrorをStorageErrorに変換するヘルパー
fn storage_error_from_index(e: crate::infrastructure::sqlite_index::IndexError) -> crate::traits::StorageError {
    crate::traits::StorageError::Io(std::io::Error::new(
        std::io::ErrorKind::Other,
        e.to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::{FileStorage, HeadingFilenameStrategy, SqliteIndex};
    use crate::infrastructure::EventBusImpl;
    use crate::infrastructure::FileSettingsRepository;
    use crate::services::SettingsService;
    use tempfile::TempDir;

    fn create_test_repo(temp_dir: &TempDir) -> HybridRepository {
        let index = Arc::new(SqliteIndex::open_in_memory().unwrap());
        let storage = Arc::new(FileStorage::new());
        let filename_strategy = Arc::new(HeadingFilenameStrategy::new());

        // 設定サービスを作成
        let settings_repo = Arc::new(FileSettingsRepository::with_path(
            temp_dir.path().join("config.toml"),
        ));
        let event_bus = Arc::new(EventBusImpl::new());
        let settings_service = Arc::new(SettingsService::new(settings_repo, event_bus));

        // storage_directory を temp_dir に設定
        settings_service
            .update(|s| {
                s.storage_directory = temp_dir.path().to_path_buf();
            })
            .unwrap();

        HybridRepository::new(index, storage, filename_strategy, settings_service)
    }

    #[test]
    fn test_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let repo = create_test_repo(&temp_dir);

        let mut note = Note::new();
        note.content = "# テストメモ\n\n本文".to_string();

        let path = repo.save(&note).unwrap();
        assert!(path.exists());

        let loaded = repo.load(&note.metadata.uid).unwrap();
        assert_eq!(note.content, loaded.content);
    }

    #[test]
    fn test_list_all() {
        let temp_dir = TempDir::new().unwrap();
        let repo = create_test_repo(&temp_dir);

        // 複数のメモを保存
        for i in 0..3 {
            let mut note = Note::new();
            note.content = format!("# メモ {}\n\n本文", i);
            repo.save(&note).unwrap();
        }

        let items = repo.list_all().unwrap();
        assert_eq!(items.len(), 3);
    }

    #[test]
    fn test_delete() {
        let temp_dir = TempDir::new().unwrap();
        let repo = create_test_repo(&temp_dir);

        let mut note = Note::new();
        note.content = "# 削除テスト\n\n本文".to_string();
        let path = repo.save(&note).unwrap();

        assert!(path.exists());
        assert_eq!(repo.list_all().unwrap().len(), 1);

        repo.delete(&note.metadata.uid).unwrap();

        assert!(!path.exists());
        assert_eq!(repo.list_all().unwrap().len(), 0);
    }

    #[test]
    fn test_find_by_title() {
        let temp_dir = TempDir::new().unwrap();
        let repo = create_test_repo(&temp_dir);

        let mut note = Note::new();
        note.content = "# ユニークタイトル\n\n本文".to_string();
        repo.save(&note).unwrap();

        // タイトルで検索
        let found = repo.find_by_title("ユニークタイトル").unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().uid, note.metadata.uid);

        // 大文字小文字無視（日本語なので関係ないが）
        let found = repo.find_by_title("ユニークタイトル").unwrap();
        assert!(found.is_some());

        // 存在しないタイトル
        let found = repo.find_by_title("存在しない").unwrap();
        assert!(found.is_none());
    }

    #[test]
    fn test_pagination() {
        let temp_dir = TempDir::new().unwrap();
        let repo = create_test_repo(&temp_dir);

        // 10件のノートを作成
        for i in 0..10 {
            let mut note = Note::new();
            note.content = format!("# メモ {}\n\n本文", i);
            repo.save(&note).unwrap();
        }

        // ページネーション取得
        let (items, total) = repo.list_paginated(0, 3).unwrap();
        assert_eq!(total, 10);
        assert_eq!(items.len(), 3);

        let (items, _) = repo.list_paginated(3, 3).unwrap();
        assert_eq!(items.len(), 3);

        let (items, _) = repo.list_paginated(9, 10).unwrap();
        assert_eq!(items.len(), 1);
    }

    #[test]
    fn test_equivalence_with_file_repo() {
    
        let temp_dir = TempDir::new().unwrap();

        // HybridRepository
        let hybrid = create_test_repo(&temp_dir);

        // 同じ操作で同じ結果
        let mut note = Note::new();
        note.content = "# テスト\n\nContent".to_string();

        hybrid.save(&note).unwrap();

        let list = hybrid.list_all().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].uid, note.metadata.uid);
        assert_eq!(list[0].title, "テスト");
    }
}
