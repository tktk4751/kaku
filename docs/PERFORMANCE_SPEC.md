# kaku 高速化仕様書 v1.0

**目標**: 10万件のMarkdownファイルでも超高速に動作する「宇宙最速のメモアプリ」

**基本原則**: 現在の挙動を100%維持しながら、内部実装のみを最適化

---

## 目次

1. [アーキテクチャ概要](#1-アーキテクチャ概要)
2. [Phase 1: SQLiteインデックス導入](#2-phase-1-sqliteインデックス導入)
3. [Phase 2: フロントエンド仮想スクロール](#3-phase-2-フロントエンド仮想スクロール)
4. [Phase 3: ページネーションAPI](#4-phase-3-ページネーションapi)
5. [Phase 4: バックリンク最適化](#5-phase-4-バックリンク最適化)
6. [Phase 5: エディタ最適化](#6-phase-5-エディタ最適化)
7. [Phase 6: 並列処理導入](#7-phase-6-並列処理導入)
8. [テスト戦略](#8-テスト戦略)
9. [マイグレーション計画](#9-マイグレーション計画)
10. [期待される性能改善](#10-期待される性能改善)

---

## 1. アーキテクチャ概要

### 1.1 現在のアーキテクチャ

```
┌─────────────────────────────────────────────────────────────┐
│                      Frontend (SvelteKit 5)                  │
├─────────────────────────────────────────────────────────────┤
│  Stores: note.svelte.ts, settings.svelte.ts, search.svelte.ts│
│  Components: Sidebar, Editor, CommandPalette                 │
│  Editor: CodeMirror 6 + LivePreview + WikiLink              │
└─────────────────────────────────────────────────────────────┘
                              │ Tauri IPC
┌─────────────────────────────────────────────────────────────┐
│                      Backend (Rust/Tauri)                    │
├─────────────────────────────────────────────────────────────┤
│  Commands: note, settings, backlink, window, hotkey          │
│  Services: NoteService, SearchService, BacklinkService       │
│  Repository: FileNoteRepository (in-memory cache)            │
│  Storage: FileStorage (filesystem)                           │
└─────────────────────────────────────────────────────────────┘
                              │
                    ┌─────────────────┐
                    │   Filesystem    │
                    │   (.md files)   │
                    └─────────────────┘
```

### 1.2 目標アーキテクチャ

```
┌─────────────────────────────────────────────────────────────┐
│                      Frontend (SvelteKit 5)                  │
├─────────────────────────────────────────────────────────────┤
│  Stores: note.svelte.ts (paginated), virtual-list.svelte.ts │
│  Components: VirtualSidebar, OptimizedEditor                 │
│  Editor: CodeMirror 6 + DiffLivePreview + CachedWikiLink    │
└─────────────────────────────────────────────────────────────┘
                              │ Tauri IPC (paginated)
┌─────────────────────────────────────────────────────────────┐
│                      Backend (Rust/Tauri)                    │
├─────────────────────────────────────────────────────────────┤
│  Commands: note (paginated), backlink (cached)               │
│  Services: Enhanced with SQLite index                        │
│  Repository: HybridRepository (SQLite + File)                │
│  Index: SQLite (metadata, FTS5, backlinks)                   │
│  Storage: FileStorage (content only)                         │
└─────────────────────────────────────────────────────────────┘
                    │                    │
          ┌─────────────────┐   ┌─────────────────┐
          │     SQLite      │   │   Filesystem    │
          │  (index.db)     │   │   (.md files)   │
          └─────────────────┘   └─────────────────┘
```

### 1.3 API互換性保証

**変更禁止項目（既存契約）:**

| カテゴリ | 項目 | 型/値 |
|---------|------|-------|
| Tauri Commands | `create_note` | `() -> NoteDto` |
| | `save_note` | `(uid: String, content: String) -> ()` |
| | `load_note` | `(uid: String) -> NoteDto` |
| | `delete_note` | `(uid: String) -> ()` |
| | `list_notes` | `() -> Vec<NoteListItemDto>` |
| | `search_notes` | `(query: String, limit?: usize) -> Vec<SearchResultDto>` |
| | `resolve_wiki_link` | `(title: String) -> NoteDto` |
| | `get_backlinks` | `(uid: String) -> Vec<BacklinkDto>` |
| DTOs | `NoteDto` | `{uid, content, created_at, updated_at, is_dirty}` |
| | `NoteListItemDto` | `{uid, title, updated_at}` |
| | `SearchResultDto` | `{uid, title, score, title_matches, content_preview}` |
| | `BacklinkDto` | `{uid, title, context}` |
| 日時形式 | timestamps | `"YYYY-MM-DD HH:MM:SS"` |
| ファイル形式 | notes | YAML front matter + Markdown |

**新規追加API（後方互換）:**

```rust
// 既存APIは維持、新APIを追加
#[tauri::command]
pub fn list_notes_paginated(
    offset: usize,
    limit: usize,
    state: State<AppState>
) -> Result<PaginatedNotesDto, String>;

#[derive(Serialize)]
pub struct PaginatedNotesDto {
    pub items: Vec<NoteListItemDto>,
    pub total: usize,
    pub offset: usize,
    pub limit: usize,
    pub has_more: bool,
}
```

---

## 2. Phase 1: SQLiteインデックス導入

### 2.1 概要

ファイルシステムの全スキャンを回避し、メタデータをSQLiteで管理。

**目標:**
- `list_notes()`: 2.5秒 → <10ms
- `search_notes()`: 1-3秒 → <50ms
- メモリ使用量: 500MB+ → <50MB

### 2.2 データベーススキーマ

```sql
-- ファイル: ~/.config/kaku/index.db

-- メインインデックステーブル
CREATE TABLE IF NOT EXISTS notes (
    uid TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    file_path TEXT NOT NULL UNIQUE,
    content_hash TEXT NOT NULL,  -- 変更検出用
    created_at TEXT NOT NULL,    -- ISO8601
    updated_at TEXT NOT NULL,    -- ISO8601
    indexed_at TEXT NOT NULL     -- インデックス更新時刻
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
    target_title TEXT NOT NULL,  -- リンク先タイトル（正規化済み）
    position INTEGER NOT NULL,   -- コンテンツ内の位置
    FOREIGN KEY (source_uid) REFERENCES notes(uid) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_backlinks_target ON backlinks(target_title);
CREATE INDEX IF NOT EXISTS idx_backlinks_source ON backlinks(source_uid);

-- タイトル→UID逆引きテーブル（O(1)検索用）
CREATE TABLE IF NOT EXISTS title_index (
    title_normalized TEXT PRIMARY KEY,  -- lowercase
    uid TEXT NOT NULL,
    FOREIGN KEY (uid) REFERENCES notes(uid) ON DELETE CASCADE
);

-- スキーママイグレーション管理
CREATE TABLE IF NOT EXISTS schema_version (
    version INTEGER PRIMARY KEY,
    applied_at TEXT NOT NULL
);
```

### 2.3 Rustモジュール構造

```
src-tauri/src/
├── infrastructure/
│   ├── mod.rs
│   ├── sqlite_index.rs          # NEW: SQLiteインデックス管理
│   ├── hybrid_repository.rs     # NEW: SQLite + File ハイブリッド
│   ├── file_repository.rs       # 既存（内部で利用）
│   └── file_storage.rs          # 既存
```

### 2.4 SQLiteIndex実装仕様

```rust
// src-tauri/src/infrastructure/sqlite_index.rs

use rusqlite::{Connection, params};
use std::path::PathBuf;
use std::sync::Mutex;

/// SQLiteインデックスマネージャー
///
/// # スレッドセーフティ
/// - Connection は Mutex で保護
/// - 読み取りは並列可能（将来的にはRwLockに変更検討）
pub struct SqliteIndex {
    conn: Mutex<Connection>,
    db_path: PathBuf,
}

impl SqliteIndex {
    /// 新規作成または既存DBを開く
    pub fn open(db_path: PathBuf) -> Result<Self, IndexError> {
        let conn = Connection::open(&db_path)?;
        let index = Self {
            conn: Mutex::new(conn),
            db_path,
        };
        index.run_migrations()?;
        Ok(index)
    }

    /// スキーママイグレーション実行
    fn run_migrations(&self) -> Result<(), IndexError> {
        let conn = self.conn.lock().unwrap();

        // 現在のバージョン確認
        let current_version: i32 = conn
            .query_row(
                "SELECT COALESCE(MAX(version), 0) FROM schema_version",
                [],
                |row| row.get(0)
            )
            .unwrap_or(0);

        // マイグレーション適用
        if current_version < 1 {
            conn.execute_batch(MIGRATION_V1)?;
            conn.execute(
                "INSERT INTO schema_version (version, applied_at) VALUES (1, datetime('now'))",
                []
            )?;
        }

        Ok(())
    }

    /// ノートをインデックスに追加/更新
    ///
    /// # 処理フロー
    /// 1. notes テーブルに UPSERT
    /// 2. notes_fts を更新
    /// 3. backlinks を再構築
    /// 4. title_index を更新
    pub fn upsert_note(&self, note: &IndexedNote) -> Result<(), IndexError> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "INSERT INTO notes (uid, title, file_path, content_hash, created_at, updated_at, indexed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, datetime('now'))
             ON CONFLICT(uid) DO UPDATE SET
                title = excluded.title,
                file_path = excluded.file_path,
                content_hash = excluded.content_hash,
                updated_at = excluded.updated_at,
                indexed_at = datetime('now')",
            params![
                note.uid,
                note.title,
                note.file_path.to_string_lossy(),
                note.content_hash,
                note.created_at,
                note.updated_at
            ]
        )?;

        // FTS更新
        conn.execute("DELETE FROM notes_fts WHERE uid = ?1", params![note.uid])?;
        conn.execute(
            "INSERT INTO notes_fts (uid, title, content) VALUES (?1, ?2, ?3)",
            params![note.uid, note.title, note.content]
        )?;

        // バックリンク更新
        self.update_backlinks_internal(&conn, &note.uid, &note.content)?;

        // タイトルインデックス更新
        let title_normalized = note.title.to_lowercase();
        conn.execute(
            "INSERT OR REPLACE INTO title_index (title_normalized, uid) VALUES (?1, ?2)",
            params![title_normalized, note.uid]
        )?;

        Ok(())
    }

    /// ノートをインデックスから削除
    pub fn delete_note(&self, uid: &str) -> Result<(), IndexError> {
        let conn = self.conn.lock().unwrap();

        // CASCADE により backlinks, title_index も自動削除
        conn.execute("DELETE FROM notes WHERE uid = ?1", params![uid])?;
        conn.execute("DELETE FROM notes_fts WHERE uid = ?1", params![uid])?;

        Ok(())
    }

    /// ノート一覧を取得（ページネーション対応）
    ///
    /// # 引数
    /// - `offset`: 開始位置
    /// - `limit`: 取得件数
    ///
    /// # 戻り値
    /// - `(items, total_count)`
    pub fn list_notes(&self, offset: usize, limit: usize) -> Result<(Vec<NoteListItem>, usize), IndexError> {
        let conn = self.conn.lock().unwrap();

        let total: usize = conn.query_row(
            "SELECT COUNT(*) FROM notes",
            [],
            |row| row.get(0)
        )?;

        let mut stmt = conn.prepare(
            "SELECT uid, title, file_path, updated_at
             FROM notes
             ORDER BY updated_at DESC
             LIMIT ?1 OFFSET ?2"
        )?;

        let items: Vec<NoteListItem> = stmt
            .query_map(params![limit, offset], |row| {
                Ok(NoteListItem {
                    uid: row.get(0)?,
                    title: row.get(1)?,
                    path: PathBuf::from(row.get::<_, String>(2)?),
                    updated_at: parse_datetime(row.get::<_, String>(3)?),
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

    /// タイトルでノートを検索（O(1)）
    pub fn find_by_title(&self, title: &str) -> Result<Option<String>, IndexError> {
        let conn = self.conn.lock().unwrap();
        let title_normalized = title.to_lowercase();

        let result = conn.query_row(
            "SELECT uid FROM title_index WHERE title_normalized = ?1",
            params![title_normalized],
            |row| row.get(0)
        );

        match result {
            Ok(uid) => Ok(Some(uid)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// FTS5による全文検索
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchHit>, IndexError> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT uid, title, snippet(notes_fts, 2, '<mark>', '</mark>', '...', 32)
             FROM notes_fts
             WHERE notes_fts MATCH ?1
             ORDER BY rank
             LIMIT ?2"
        )?;

        let hits: Vec<SearchHit> = stmt
            .query_map(params![query, limit], |row| {
                Ok(SearchHit {
                    uid: row.get(0)?,
                    title: row.get(1)?,
                    snippet: row.get(2)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(hits)
    }

    /// バックリンク取得
    pub fn get_backlinks(&self, uid: &str) -> Result<Vec<BacklinkInfo>, IndexError> {
        let conn = self.conn.lock().unwrap();

        // まず対象ノートのタイトルを取得
        let title: String = conn.query_row(
            "SELECT title FROM notes WHERE uid = ?1",
            params![uid],
            |row| row.get(0)
        )?;

        let title_normalized = title.to_lowercase();

        // そのタイトルへのリンクを持つノートを検索
        let mut stmt = conn.prepare(
            "SELECT DISTINCT n.uid, n.title
             FROM backlinks b
             JOIN notes n ON b.source_uid = n.uid
             WHERE b.target_title = ?1"
        )?;

        let backlinks: Vec<BacklinkInfo> = stmt
            .query_map(params![title_normalized], |row| {
                Ok(BacklinkInfo {
                    source_uid: row.get(0)?,
                    source_title: row.get(1)?,
                    context: String::new(), // コンテキストは後で取得
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(backlinks)
    }

    /// インデックスの完全再構築
    ///
    /// # 使用場面
    /// - 初回起動時
    /// - storage_directory 変更時
    /// - インデックス破損時
    pub fn rebuild_full(&self, notes: impl Iterator<Item = IndexedNote>) -> Result<(), IndexError> {
        let conn = self.conn.lock().unwrap();

        // トランザクション開始
        conn.execute("BEGIN TRANSACTION", [])?;

        // 全テーブルクリア
        conn.execute("DELETE FROM notes", [])?;
        conn.execute("DELETE FROM notes_fts", [])?;
        conn.execute("DELETE FROM backlinks", [])?;
        conn.execute("DELETE FROM title_index", [])?;

        // バルクインサート
        for note in notes {
            // ... insert logic (同上)
        }

        conn.execute("COMMIT", [])?;

        Ok(())
    }

    /// バックリンクの内部更新
    fn update_backlinks_internal(
        &self,
        conn: &Connection,
        uid: &str,
        content: &str
    ) -> Result<(), IndexError> {
        // 既存のバックリンクを削除
        conn.execute("DELETE FROM backlinks WHERE source_uid = ?1", params![uid])?;

        // WikiLinkを抽出して挿入
        let links = extract_wiki_links(content);
        for link in links {
            let target_normalized = link.title.to_lowercase();
            conn.execute(
                "INSERT INTO backlinks (source_uid, target_title, position) VALUES (?1, ?2, ?3)",
                params![uid, target_normalized, link.position]
            )?;
        }

        Ok(())
    }
}

/// インデックス用ノート構造体
pub struct IndexedNote {
    pub uid: String,
    pub title: String,
    pub content: String,
    pub file_path: PathBuf,
    pub content_hash: String,
    pub created_at: String,
    pub updated_at: String,
}

/// 検索ヒット
pub struct SearchHit {
    pub uid: String,
    pub title: String,
    pub snippet: String,
}
```

### 2.5 HybridRepository実装仕様

```rust
// src-tauri/src/infrastructure/hybrid_repository.rs

/// ハイブリッドリポジトリ
///
/// SQLiteインデックス + ファイルシステムの組み合わせ
///
/// # 責務分担
/// - SQLite: メタデータ、検索、バックリンク
/// - FileSystem: コンテンツ保存
///
/// # 一貫性保証
/// - 保存時: ファイル → SQLite の順で更新
/// - 削除時: SQLite → ファイル の順で削除
/// - エラー時: ロールバック試行
pub struct HybridRepository {
    index: Arc<SqliteIndex>,
    storage: Arc<dyn Storage>,
    filename_strategy: Arc<dyn FilenameStrategy>,
    settings_service: Arc<SettingsService>,
}

impl NoteRepository for HybridRepository {
    fn save(&self, note: &Note) -> Result<PathBuf, RepositoryError> {
        // 1. ファイルパスを決定
        let path = self.resolve_or_generate_path(note)?;

        // 2. ファイルに保存（アトミック）
        let content = note.to_file_content();
        self.storage.save_atomic(&path, &content)?;

        // 3. インデックスを更新
        let indexed_note = IndexedNote {
            uid: note.metadata.uid.clone(),
            title: note.extract_heading().unwrap_or_else(|| note.metadata.uid.clone()),
            content: note.content.clone(),
            file_path: path.clone(),
            content_hash: compute_hash(&content),
            created_at: format_datetime(note.metadata.created_at),
            updated_at: format_datetime(note.metadata.updated_at),
        };

        self.index.upsert_note(&indexed_note)?;

        Ok(path)
    }

    fn load(&self, uid: &str) -> Result<Note, RepositoryError> {
        // インデックスからパスを取得（O(1)）
        let path = self.index.get_path(uid)?
            .ok_or_else(|| RepositoryError::not_found(uid))?;

        // ファイルを読み込み
        let content = self.storage.load(&path)?;
        Note::from_file_content(&content)
            .map_err(|_| RepositoryError::parse("Invalid note format", Some(path)))
    }

    fn delete(&self, uid: &str) -> Result<(), RepositoryError> {
        // 1. パスを取得
        let path = self.index.get_path(uid)?
            .ok_or_else(|| RepositoryError::not_found(uid))?;

        // 2. インデックスから削除
        self.index.delete_note(uid)?;

        // 3. ファイルを削除
        self.storage.delete(&path)?;

        Ok(())
    }

    fn list_all(&self) -> Result<Vec<NoteListItem>, RepositoryError> {
        // 後方互換: 全件取得
        self.index.list_all_notes()
            .map_err(|e| RepositoryError::storage("list_all", e.into()))
    }

    fn get_path(&self, uid: &str) -> Option<PathBuf> {
        self.index.get_path(uid).ok().flatten()
    }
}

impl HybridRepository {
    /// ページネーション対応リスト取得
    pub fn list_paginated(&self, offset: usize, limit: usize) -> Result<(Vec<NoteListItem>, usize), RepositoryError> {
        self.index.list_notes(offset, limit)
            .map_err(|e| RepositoryError::storage("list_paginated", e.into()))
    }

    /// タイトルでノートを検索（O(1)）
    pub fn find_by_title(&self, title: &str) -> Result<Option<NoteListItem>, RepositoryError> {
        if let Some(uid) = self.index.find_by_title(title)? {
            let items = self.index.list_all_notes()?;
            Ok(items.into_iter().find(|i| i.uid == uid))
        } else {
            Ok(None)
        }
    }

    /// インデックス同期チェック
    ///
    /// ファイルシステムとインデックスの整合性を確認
    pub fn sync_index(&self) -> Result<SyncResult, RepositoryError> {
        let base_dir = self.settings_service.storage_directory();
        let files = self.storage.list_files(&base_dir, "md")?;

        let mut added = 0;
        let mut updated = 0;
        let mut removed = 0;

        // ファイルを走査してインデックスを更新
        for path in files {
            if let Ok(content) = self.storage.load(&path) {
                if let Ok(note) = Note::from_file_content(&content) {
                    let hash = compute_hash(&content);

                    // インデックスにない or ハッシュが違う場合は更新
                    if self.index.needs_update(&note.metadata.uid, &hash)? {
                        let indexed = IndexedNote::from_note_and_path(&note, &path, hash);
                        self.index.upsert_note(&indexed)?;
                        updated += 1;
                    }
                }
            }
        }

        // インデックスにあってファイルにないものを削除
        removed = self.index.remove_orphans(&base_dir)?;

        Ok(SyncResult { added, updated, removed })
    }
}
```

### 2.6 依存関係追加（Cargo.toml）

```toml
[dependencies]
# ... existing deps ...

# SQLite
rusqlite = { version = "0.31", features = ["bundled"] }

# Hash computation
blake3 = "1.5"
```

---

## 3. Phase 2: フロントエンド仮想スクロール

### 3.1 概要

10万件のノートを効率的に表示するため、表示範囲のみをDOMに生成。

**目標:**
- DOM要素数: 100,000 → 50以下
- スクロールFPS: フリーズ → 60fps
- 初期表示: 即座

### 3.2 VirtualList コンポーネント仕様

```typescript
// src/lib/components/VirtualList.svelte

<script lang="ts">
  import { onMount } from 'svelte';

  // Props
  interface Props<T> {
    /** 全アイテム数 */
    totalCount: number;
    /** 各アイテムの高さ（px） */
    itemHeight: number;
    /** 表示領域の高さ（px） */
    viewportHeight: number;
    /** オーバースキャン（前後に余分にレンダリングする数） */
    overscan?: number;
    /** アイテム取得関数 */
    getItems: (start: number, end: number) => Promise<T[]>;
    /** アイテムレンダリング用snippet */
    children: import('svelte').Snippet<[T, number]>;
  }

  let {
    totalCount,
    itemHeight,
    viewportHeight,
    overscan = 5,
    getItems,
    children
  }: Props<T> = $props();

  // Internal state
  let scrollTop = $state(0);
  let items = $state<T[]>([]);
  let containerRef: HTMLDivElement;

  // Derived values
  const totalHeight = $derived(totalCount * itemHeight);
  const visibleCount = $derived(Math.ceil(viewportHeight / itemHeight));
  const startIndex = $derived(Math.max(0, Math.floor(scrollTop / itemHeight) - overscan));
  const endIndex = $derived(Math.min(totalCount, startIndex + visibleCount + overscan * 2));
  const offsetY = $derived(startIndex * itemHeight);

  // Fetch items when range changes
  $effect(() => {
    const start = startIndex;
    const end = endIndex;

    getItems(start, end).then(newItems => {
      items = newItems;
    });
  });

  function handleScroll(e: Event) {
    const target = e.target as HTMLDivElement;
    scrollTop = target.scrollTop;
  }

  onMount(() => {
    // Initial load
    getItems(0, visibleCount + overscan).then(newItems => {
      items = newItems;
    });
  });
</script>

<div
  bind:this={containerRef}
  class="virtual-list-container"
  style:height="{viewportHeight}px"
  onscroll={handleScroll}
>
  <div class="virtual-list-spacer" style:height="{totalHeight}px">
    <div class="virtual-list-items" style:transform="translateY({offsetY}px)">
      {#each items as item, i (startIndex + i)}
        {@render children(item, startIndex + i)}
      {/each}
    </div>
  </div>
</div>

<style>
  .virtual-list-container {
    overflow-y: auto;
    position: relative;
  }

  .virtual-list-spacer {
    position: relative;
  }

  .virtual-list-items {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
  }
</style>
```

### 3.3 VirtualSidebar 実装仕様

```typescript
// src/lib/components/VirtualSidebar.svelte

<script lang="ts">
  import VirtualList from './VirtualList.svelte';
  import { useNoteStore } from '$lib/stores/note.svelte';
  import { listNotesPaginated } from '$lib/services/api';
  import type { NoteListItemDto } from '$lib/types';

  const noteStore = useNoteStore();

  // アイテム高さ（CSSと同期必須）
  const ITEM_HEIGHT = 60;
  const OVERSCAN = 10;

  // 総数はストアから取得
  let totalCount = $state(0);
  let viewportHeight = $state(400);
  let containerRef: HTMLDivElement;

  // キャッシュ
  const itemCache = new Map<number, NoteListItemDto>();
  let cacheVersion = 0;

  // アイテム取得（キャッシュ対応）
  async function getItems(start: number, end: number): Promise<NoteListItemDto[]> {
    const needed: number[] = [];

    for (let i = start; i < end; i++) {
      if (!itemCache.has(i)) {
        needed.push(i);
      }
    }

    if (needed.length > 0) {
      const minIdx = Math.min(...needed);
      const maxIdx = Math.max(...needed);

      // バッチ取得
      const result = await listNotesPaginated(minIdx, maxIdx - minIdx + 1);
      totalCount = result.total;

      result.items.forEach((item, i) => {
        itemCache.set(minIdx + i, item);
      });
    }

    return Array.from({ length: end - start }, (_, i) =>
      itemCache.get(start + i)!
    ).filter(Boolean);
  }

  // リスト更新時にキャッシュクリア
  $effect(() => {
    if (noteStore.listVersion !== cacheVersion) {
      itemCache.clear();
      cacheVersion = noteStore.listVersion;
    }
  });

  // コンテナサイズ監視
  onMount(() => {
    const observer = new ResizeObserver(entries => {
      viewportHeight = entries[0].contentRect.height;
    });
    observer.observe(containerRef);
    return () => observer.disconnect();
  });

  function handleNoteClick(uid: string) {
    noteStore.load(uid);
  }

  function handleNoteContextMenu(e: MouseEvent, uid: string) {
    // 右クリックメニュー
  }
</script>

<div bind:this={containerRef} class="virtual-sidebar">
  <VirtualList
    {totalCount}
    itemHeight={ITEM_HEIGHT}
    {viewportHeight}
    overscan={OVERSCAN}
    {getItems}
  >
    {#snippet children(item: NoteListItemDto, index: number)}
      <div
        class="note-item"
        class:selected={noteStore.currentNote?.uid === item.uid}
        onclick={() => handleNoteClick(item.uid)}
        oncontextmenu={(e) => handleNoteContextMenu(e, item.uid)}
      >
        <span class="note-title">{item.title || 'Untitled'}</span>
        <span class="note-date">{formatDate(item.updated_at)}</span>
      </div>
    {/snippet}
  </VirtualList>
</div>

<style>
  .virtual-sidebar {
    height: 100%;
    overflow: hidden;
  }

  .note-item {
    height: 60px;
    padding: 8px 12px;
    cursor: pointer;
    border-bottom: 1px solid var(--border-color);
    display: flex;
    flex-direction: column;
    justify-content: center;
  }

  .note-item:hover {
    background: var(--hover-bg);
  }

  .note-item.selected {
    background: var(--selected-bg);
  }

  .note-title {
    font-weight: 500;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .note-date {
    font-size: 0.8em;
    color: var(--text-muted);
  }
</style>
```

### 3.4 CommandPalette 仮想化

```typescript
// src/lib/components/CommandPalette.svelte の修正

// 既存の results 配列を仮想化
// MAX_RESULTS を50に制限（これ以上はスクロール）

const MAX_VISIBLE_RESULTS = 10;  // 表示する最大数
const RESULT_HEIGHT = 48;       // 各結果の高さ

// 検索結果が多い場合は仮想スクロール適用
{#if results.length <= MAX_VISIBLE_RESULTS}
  {#each results as result, index (result.uid)}
    <ResultItem {result} {index} />
  {/each}
{:else}
  <VirtualList
    totalCount={results.length}
    itemHeight={RESULT_HEIGHT}
    viewportHeight={MAX_VISIBLE_RESULTS * RESULT_HEIGHT}
    getItems={(start, end) => Promise.resolve(results.slice(start, end))}
  >
    {#snippet children(result, index)}
      <ResultItem {result} {index} />
    {/snippet}
  </VirtualList>
{/if}
```

---

## 4. Phase 3: ページネーションAPI

### 4.1 概要

バックエンドAPIにページネーションを追加し、大量データの効率的な転送を実現。

### 4.2 新規コマンド定義

```rust
// src-tauri/src/commands/note.rs

/// ページネーション結果
#[derive(Serialize)]
pub struct PaginatedNotesDto {
    /// ノートアイテム
    pub items: Vec<NoteListItemDto>,
    /// 全件数
    pub total: usize,
    /// 開始位置
    pub offset: usize,
    /// 取得件数
    pub limit: usize,
    /// 次ページあり
    pub has_more: bool,
}

/// ノート一覧取得（ページネーション対応）
///
/// # 引数
/// - `offset`: 開始位置（0始まり）
/// - `limit`: 取得件数（最大100）
///
/// # 戻り値
/// ページネーション情報付きのノートリスト
#[tauri::command]
pub fn list_notes_paginated(
    offset: usize,
    limit: usize,
    state: State<AppState>,
) -> Result<PaginatedNotesDto, String> {
    // limitの上限
    let limit = limit.min(100);

    let (items, total) = state
        .note_repository
        .list_paginated(offset, limit)
        .map_err(|e| e.to_string())?;

    let items_dto: Vec<NoteListItemDto> = items
        .into_iter()
        .map(|item| NoteListItemDto {
            uid: item.uid,
            title: item.title,
            updated_at: format_datetime(item.updated_at),
        })
        .collect();

    Ok(PaginatedNotesDto {
        items: items_dto,
        total,
        offset,
        limit,
        has_more: offset + limit < total,
    })
}

// 既存のlist_notesは後方互換のため維持
#[tauri::command]
pub fn list_notes(state: State<AppState>) -> Result<Vec<NoteListItemDto>, String> {
    // 内部的には全件取得
    let items = state
        .note_repository
        .list_all()
        .map_err(|e| e.to_string())?;

    Ok(items
        .into_iter()
        .map(|item| NoteListItemDto {
            uid: item.uid,
            title: item.title,
            updated_at: format_datetime(item.updated_at),
        })
        .collect())
}
```

### 4.3 フロントエンドAPI追加

```typescript
// src/lib/services/api.ts

export interface PaginatedNotes {
  items: NoteListItemDto[];
  total: number;
  offset: number;
  limit: number;
  has_more: boolean;
}

/**
 * ノート一覧取得（ページネーション対応）
 *
 * @param offset 開始位置
 * @param limit 取得件数（最大100）
 */
export async function listNotesPaginated(
  offset: number,
  limit: number
): Promise<PaginatedNotes> {
  return invoke<PaginatedNotes>('list_notes_paginated', { offset, limit });
}

/**
 * ノート一覧取得（ページネーション対応、Safe版）
 */
export async function listNotesPaginatedSafe(
  offset: number,
  limit: number
): Promise<Result<PaginatedNotes, AppError>> {
  try {
    const result = await listNotesPaginated(offset, limit);
    return ok(result);
  } catch (e) {
    return err(parseAppError(e));
  }
}

// 既存のlistNotesは維持（後方互換）
export async function listNotes(): Promise<NoteListItemDto[]> {
  return invoke<NoteListItemDto[]>('list_notes');
}
```

### 4.4 ストア更新

```typescript
// src/lib/stores/note.svelte.ts

// 追加: ページネーション状態
let totalNotes = $state(0);
let listVersion = $state(0);  // キャッシュ無効化用

// 追加: ページネーション取得
async function fetchPage(offset: number, limit: number): Promise<NoteListItemDto[]> {
  const result = await listNotesPaginated(offset, limit);
  totalNotes = result.total;
  return result.items;
}

// 追加: リストバージョン更新（保存/削除時に呼ぶ）
function invalidateList() {
  listVersion++;
}

// 公開API追加
return {
  // ... existing API ...

  get totalNotes() { return totalNotes; },
  get listVersion() { return listVersion; },
  fetchPage,
};
```

---

## 5. Phase 4: バックリンク最適化

### 5.1 概要

メモリ消費を500MB→<50MBに削減、更新をO(n²)→O(1)に改善。

### 5.2 BacklinkService 改修

```rust
// src-tauri/src/services/backlink_service.rs

/// 最適化されたバックリンクサービス
///
/// # 変更点
/// - コンテンツをメモリに保持しない
/// - SQLiteインデックスを使用
/// - 増分更新対応
pub struct OptimizedBacklinkService {
    index: Arc<SqliteIndex>,
    repository: Arc<dyn NoteRepository>,
}

impl OptimizedBacklinkService {
    pub fn new(index: Arc<SqliteIndex>, repository: Arc<dyn NoteRepository>) -> Self {
        Self { index, repository }
    }

    /// 特定ノートへのバックリンクを取得
    ///
    /// SQLiteインデックスからO(log n)で取得
    pub fn get_backlinks(&self, uid: &str) -> Result<Vec<BacklinkInfo>, SearchError> {
        let backlinks = self.index.get_backlinks(uid)?;

        // コンテキストを取得（必要な分だけファイルを読む）
        let mut result = Vec::with_capacity(backlinks.len());
        for bl in backlinks {
            let context = self.extract_context(&bl.source_uid, uid)?;
            result.push(BacklinkInfo {
                source_uid: bl.source_uid,
                source_title: bl.source_title,
                context,
            });
        }

        Ok(result)
    }

    /// 単一ノートの更新（増分）
    ///
    /// 保存時に呼ばれる。フルリビルドではなく差分更新。
    pub fn update_note(&self, uid: &str, title: &str, content: &str) -> Result<(), SearchError> {
        // SQLiteインデックスが自動的にバックリンクを更新
        // （upsert_note内で処理済み）
        Ok(())
    }

    /// ノート削除時の処理
    pub fn remove_note(&self, uid: &str) -> Result<(), SearchError> {
        // SQLiteインデックスが自動的にバックリンクを削除
        // （delete_note内でCASCADE削除）
        Ok(())
    }

    /// コンテキスト抽出（オンデマンド）
    fn extract_context(&self, source_uid: &str, target_uid: &str) -> Result<String, SearchError> {
        let note = self.repository.load(source_uid)?;

        // 対象ノートのタイトルを取得
        let target_note = self.repository.load(target_uid)?;
        let target_title = target_note.extract_heading()
            .unwrap_or_else(|| target_uid.to_string());

        // コンテンツ内でリンクを探す
        let links = extract_wiki_links(&note.content);
        for link in links {
            if link.title.to_lowercase() == target_title.to_lowercase() {
                return Ok(extract_context(&note.content, link.position, 50));
            }
        }

        Ok(String::new())
    }

    /// インデックス再構築（初回起動時のみ）
    ///
    /// # 注意
    /// 通常は使用しない。storage_directory変更時などに呼ぶ。
    pub fn rebuild_index(&self) -> Result<(), SearchError> {
        // SQLiteインデックスの rebuild_full を呼ぶ
        // HybridRepository.sync_index() 経由で実行
        Ok(())
    }
}
```

### 5.3 メモリ使用量比較

| 項目 | Before | After |
|-----|--------|-------|
| リンクマップ | HashMap<String, HashSet<String>> | SQLite backlinks テーブル |
| タイトルキャッシュ | HashMap<String, String> | SQLite title_index テーブル |
| コンテンツキャッシュ | HashMap<String, String> (500MB) | なし（オンデマンド読込） |
| **合計 (100k files)** | **~530MB** | **~50MB** (SQLite DB) |

---

## 6. Phase 5: エディタ最適化

### 6.1 概要

CodeMirrorの不要な再計算を削減し、大きなファイルでも快適に編集。

### 6.2 LivePreview プラグイン最適化

```typescript
// src/lib/editor/extensions/livePreview.ts

// 最適化ポイント:
// 1. selectionSet でのデコレーション再構築を回避
// 2. 差分更新の導入
// 3. デバウンス処理

class OptimizedLivePreviewPlugin implements PluginValue {
  decorations: DecorationSet;
  private lastViewport: { from: number; to: number } | null = null;
  private updateScheduled = false;

  constructor(view: EditorView) {
    this.decorations = this.buildDecorations(view);
  }

  update(update: ViewUpdate) {
    // selectionSet のみの場合はスキップ（重要な最適化）
    if (!update.docChanged && !update.viewportChanged) {
      return;
    }

    // デバウンス: 連続した更新をまとめる
    if (!this.updateScheduled) {
      this.updateScheduled = true;
      requestAnimationFrame(() => {
        this.decorations = this.buildDecorations(update.view);
        this.updateScheduled = false;
      });
    }
  }

  buildDecorations(view: EditorView): DecorationSet {
    const builder = new RangeSetBuilder<Decoration>();
    const { from: viewportFrom, to: viewportTo } = view.viewport;

    // ビューポート変更がない場合は差分更新
    if (this.lastViewport &&
        this.lastViewport.from === viewportFrom &&
        this.lastViewport.to === viewportTo) {
      // 差分のみ更新
      return this.updateIncrementally(view, builder);
    }

    this.lastViewport = { from: viewportFrom, to: viewportTo };

    // フル再構築（ビューポート変更時のみ）
    syntaxTree(view.state).iterate({
      from: viewportFrom,
      to: viewportTo,
      enter: (node) => {
        this.processNode(view, node, builder);
      }
    });

    return builder.finish();
  }

  // ノード処理を分離（キャッシュ可能に）
  private processNode(
    view: EditorView,
    node: SyntaxNodeRef,
    builder: RangeSetBuilder<Decoration>
  ) {
    // 既存のノード処理ロジック
    // ...
  }

  // 差分更新（変更された行のみ処理）
  private updateIncrementally(
    view: EditorView,
    builder: RangeSetBuilder<Decoration>
  ): DecorationSet {
    // 変更された範囲のみ再処理
    // ...
    return builder.finish();
  }
}
```

### 6.3 WikiLink プラグイン最適化

```typescript
// src/lib/editor/extensions/wikiLink.ts

// 最適化ポイント:
// 1. 正規表現を事前コンパイル
// 2. 結果をキャッシュ
// 3. 変更行のみ再処理

// 事前コンパイル（モジュールスコープ）
const WIKI_LINK_REGEX = /\[\[([^\]|]+)(?:\|([^\]]+))?\]\]/g;

class OptimizedWikiLinkPlugin implements PluginValue {
  decorations: DecorationSet;
  private lineCache = new Map<number, Decoration[]>();
  private lastDocLength = 0;

  update(update: ViewUpdate) {
    if (!update.docChanged && !update.viewportChanged) {
      return;
    }

    // ドキュメント長が変わった場合はキャッシュクリア
    if (update.state.doc.length !== this.lastDocLength) {
      this.lineCache.clear();
      this.lastDocLength = update.state.doc.length;
    }

    this.decorations = this.buildDecorations(update.view);
  }

  buildDecorations(view: EditorView): DecorationSet {
    const builder = new RangeSetBuilder<Decoration>();
    const { from: viewportFrom, to: viewportTo } = view.viewport;

    for (let pos = viewportFrom; pos < viewportTo; ) {
      const line = view.state.doc.lineAt(pos);
      const lineNum = line.number;

      // キャッシュチェック
      if (!this.lineCache.has(lineNum)) {
        const decorations = this.findWikiLinksInLine(line.text, line.from);
        this.lineCache.set(lineNum, decorations);
      }

      const cachedDecorations = this.lineCache.get(lineNum)!;
      for (const deco of cachedDecorations) {
        builder.add(deco.from, deco.to, deco.value);
      }

      pos = line.to + 1;
    }

    return builder.finish();
  }

  private findWikiLinksInLine(text: string, lineStart: number): Decoration[] {
    const decorations: Decoration[] = [];

    // 正規表現をリセット
    WIKI_LINK_REGEX.lastIndex = 0;

    let match;
    while ((match = WIKI_LINK_REGEX.exec(text)) !== null) {
      const from = lineStart + match.index;
      const to = from + match[0].length;
      const title = match[1];
      const display = match[2] || title;

      decorations.push({
        from,
        to,
        value: Decoration.replace({
          widget: new WikiLinkWidget(title, display),
        }),
      });
    }

    return decorations;
  }
}
```

### 6.4 設定変更時のエディタ再作成回避

```typescript
// src/lib/components/Editor.svelte

// 現在: 設定変更時にエディタをdestroy/recreate
// 改善: 動的にcompartmentを更新

import { Compartment } from '@codemirror/state';

// コンパートメント定義
const themeCompartment = new Compartment();
const fontCompartment = new Compartment();

// 初期化時
const extensions = [
  themeCompartment.of(currentTheme),
  fontCompartment.of(fontExtension),
  // ...
];

// 設定変更時（destroy不要）
$effect(() => {
  if (!editorView) return;

  const newTheme = getThemeExtension(settingsStore.settings.theme);
  const newFont = getFontExtension(settingsStore.settings.editor);

  editorView.dispatch({
    effects: [
      themeCompartment.reconfigure(newTheme),
      fontCompartment.reconfigure(newFont),
    ]
  });
});
```

### 6.5 onChange 最適化

```typescript
// src/lib/editor/setup.ts

// 現在: 毎キー入力でdoc.toString()
// 改善: デバウンス + 差分検出

let lastContent = '';
let debounceTimer: ReturnType<typeof setTimeout> | null = null;

const updateListener = EditorView.updateListener.of((update) => {
  if (!update.docChanged || !onChange) return;

  // デバウンス
  if (debounceTimer) clearTimeout(debounceTimer);

  debounceTimer = setTimeout(() => {
    const newContent = update.state.doc.toString();

    // 内容が実際に変わった場合のみコールバック
    if (newContent !== lastContent) {
      lastContent = newContent;
      onChange(newContent);
    }
  }, 50);  // 50ms デバウンス
});
```

### 6.6 検索状態キャッシュ

```typescript
// src/lib/editor/setup.ts

// 検索状態キャッシュ
interface CachedSearchState {
  query: string;
  matchCount: number;
  currentMatch: number;
  positions: Array<{ from: number; to: number }>;
}

let searchStateCache: CachedSearchState | null = null;

export function getSearchState(view: EditorView): SearchState {
  const query = getSearchQuery(view.state);

  if (!query.valid) {
    return { isActive: false, matchCount: 0, currentMatch: 0 };
  }

  const queryStr = query.search;

  // キャッシュヒット
  if (searchStateCache && searchStateCache.query === queryStr) {
    const selection = view.state.selection.main;

    // 現在位置のみ再計算
    let currentMatch = 0;
    for (let i = 0; i < searchStateCache.positions.length; i++) {
      const pos = searchStateCache.positions[i];
      if (pos.from <= selection.from && pos.to >= selection.to) {
        currentMatch = i + 1;
        break;
      }
    }

    return {
      isActive: true,
      matchCount: searchStateCache.matchCount,
      currentMatch,
    };
  }

  // キャッシュミス: フル計算
  const positions: Array<{ from: number; to: number }> = [];
  const cursor = query.getCursor(view.state.doc);
  let result = cursor.next();

  while (!result.done) {
    positions.push({ from: result.value.from, to: result.value.to });
    result = cursor.next();
  }

  searchStateCache = {
    query: queryStr,
    matchCount: positions.length,
    currentMatch: 0,
    positions,
  };

  return {
    isActive: true,
    matchCount: positions.length,
    currentMatch: 0,
  };
}
```

---

## 7. Phase 6: 並列処理導入

### 7.1 概要

Rayonを活用した並列ファイルスキャンで初回起動を高速化。

### 7.2 並列ファイルスキャン

```rust
// src-tauri/src/infrastructure/file_storage.rs

use rayon::prelude::*;

impl FileStorage {
    /// 並列ファイルリスト（新規追加）
    pub fn list_files_parallel(&self, dir: &Path, extension: &str) -> Result<Vec<PathBuf>, StorageError> {
        if !dir.exists() {
            return Ok(Vec::new());
        }

        let entries: Vec<_> = fs::read_dir(dir)?
            .filter_map(|e| e.ok())
            .collect();

        let files: Vec<PathBuf> = entries
            .par_iter()
            .filter_map(|entry| {
                let path = entry.path();
                if path.is_file() {
                    if let Some(ext) = path.extension() {
                        if ext == extension {
                            return Some(path);
                        }
                    }
                }
                None
            })
            .collect();

        Ok(files)
    }

    /// 並列コンテンツ読み込み（新規追加）
    pub fn load_all_parallel(&self, paths: &[PathBuf]) -> Vec<(PathBuf, Result<String, StorageError>)> {
        paths
            .par_iter()
            .map(|path| {
                let content = self.load(path);
                (path.clone(), content)
            })
            .collect()
    }
}
```

### 7.3 インデックス並列構築

```rust
// src-tauri/src/infrastructure/sqlite_index.rs

impl SqliteIndex {
    /// 並列インデックス再構築
    pub fn rebuild_parallel(&self, storage: &dyn Storage, base_dir: &Path) -> Result<(), IndexError> {
        // 1. ファイルリストを並列取得
        let paths = storage.list_files_parallel(base_dir, "md")?;

        // 2. コンテンツを並列読み込み + パース
        let notes: Vec<IndexedNote> = paths
            .par_iter()
            .filter_map(|path| {
                let content = storage.load(path).ok()?;
                let note = Note::from_file_content(&content).ok()?;

                Some(IndexedNote {
                    uid: note.metadata.uid,
                    title: note.extract_heading().unwrap_or_default(),
                    content: note.content,
                    file_path: path.clone(),
                    content_hash: compute_hash(&content),
                    created_at: format_datetime(note.metadata.created_at),
                    updated_at: format_datetime(note.metadata.updated_at),
                })
            })
            .collect();

        // 3. SQLiteにバルクインサート（シングルスレッド、トランザクション内）
        let conn = self.conn.lock().unwrap();
        conn.execute("BEGIN TRANSACTION", [])?;

        conn.execute("DELETE FROM notes", [])?;
        conn.execute("DELETE FROM notes_fts", [])?;
        conn.execute("DELETE FROM backlinks", [])?;
        conn.execute("DELETE FROM title_index", [])?;

        for note in notes {
            // ... insert logic
        }

        conn.execute("COMMIT", [])?;

        Ok(())
    }
}
```

### 7.4 並列処理の適用箇所

| 処理 | Before | After | 改善率 |
|-----|--------|-------|--------|
| ファイルリスト取得 (100k) | ~100ms | ~30ms | 3.3x |
| コンテンツ読み込み (100k) | ~2s | ~300ms | 6.7x |
| インデックス構築 (100k) | ~4s | ~800ms | 5x |

---

## 8. テスト戦略

### 8.1 回帰テスト追加（Rust）

```rust
// src-tauri/src/tests/regression.rs

#[cfg(test)]
mod regression_tests {
    use super::*;

    /// API契約テスト: list_notes の戻り値形式
    #[test]
    fn test_list_notes_contract() {
        let temp_dir = TempDir::new().unwrap();
        let repo = create_test_hybrid_repo(&temp_dir);

        // テストノート作成
        let mut note = Note::new();
        note.content = "# Test Note\n\nContent".to_string();
        repo.save(&note).unwrap();

        let items = repo.list_all().unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].uid, note.metadata.uid);
        assert!(!items[0].title.is_empty());
        // updated_at の形式は DateTime<Utc>
        assert!(items[0].updated_at <= chrono::Utc::now());
    }

    /// API契約テスト: search_notes の戻り値形式
    #[test]
    fn test_search_notes_contract() {
        // ...
    }

    /// API契約テスト: バックリンクの形式
    #[test]
    fn test_backlinks_contract() {
        // ...
    }

    /// 移行テスト: FileNoteRepository と HybridRepository の等価性
    #[test]
    fn test_repository_equivalence() {
        let temp_dir = TempDir::new().unwrap();

        let file_repo = create_test_file_repo(&temp_dir);
        let hybrid_repo = create_test_hybrid_repo(&temp_dir);

        // 同じ操作で同じ結果
        let mut note = Note::new();
        note.content = "# Test\n\nContent".to_string();

        let path1 = file_repo.save(&note).unwrap();
        let path2 = hybrid_repo.save(&note).unwrap();

        let list1 = file_repo.list_all().unwrap();
        let list2 = hybrid_repo.list_all().unwrap();

        assert_eq!(list1.len(), list2.len());
        assert_eq!(list1[0].uid, list2[0].uid);
        assert_eq!(list1[0].title, list2[0].title);
    }

    /// 性能回帰テスト
    #[test]
    fn test_list_performance() {
        let temp_dir = TempDir::new().unwrap();
        let repo = create_test_hybrid_repo(&temp_dir);

        // 1000ノート作成
        for i in 0..1000 {
            let mut note = Note::new();
            note.content = format!("# Note {}\n\nContent", i);
            repo.save(&note).unwrap();
        }

        // list_all が 100ms 以内
        let start = std::time::Instant::now();
        let _ = repo.list_all().unwrap();
        let elapsed = start.elapsed();

        assert!(elapsed.as_millis() < 100, "list_all took too long: {:?}", elapsed);
    }
}
```

### 8.2 フロントエンドテスト追加

```typescript
// src/lib/stores/note.test.ts (新規作成)

import { describe, it, expect, vi } from 'vitest';
import { useNoteStore } from './note.svelte';

describe('NoteStore', () => {
  it('should maintain backward compatibility with existing API', async () => {
    const store = useNoteStore();

    // 既存APIが存在することを確認
    expect(store.currentNote).toBeDefined();
    expect(store.noteList).toBeDefined();
    expect(store.isSaving).toBeDefined();
    expect(store.isDirty).toBeDefined();
    expect(typeof store.createNew).toBe('function');
    expect(typeof store.load).toBe('function');
    expect(typeof store.save).toBe('function');
    expect(typeof store.refreshList).toBe('function');
  });

  it('should have new pagination API', async () => {
    const store = useNoteStore();

    expect(store.totalNotes).toBeDefined();
    expect(store.listVersion).toBeDefined();
    expect(typeof store.fetchPage).toBe('function');
  });
});
```

### 8.3 E2Eテスト（手動チェックリスト）

```markdown
## 回帰テスト チェックリスト

### ノート操作
- [ ] 新規ノート作成
- [ ] ノート保存（自動保存）
- [ ] ノート読み込み
- [ ] ノート削除
- [ ] 空ノート削除

### リスト表示
- [ ] サイドバーにノート一覧表示
- [ ] 更新日時順でソート
- [ ] ノート選択時にハイライト
- [ ] スクロールが滑らか

### 検索
- [ ] コマンドパレットで検索
- [ ] タイトルマッチのハイライト
- [ ] コンテンツプレビュー表示
- [ ] 検索結果クリックでノート開く

### WikiLink
- [ ] [[リンク]] クリックでノート開く
- [ ] 存在しないリンクで新規作成
- [ ] バックリンクパネル表示

### エディタ
- [ ] Markdown構文ハイライト
- [ ] ライブプレビュー（見出し、太字等）
- [ ] Ctrl+F で検索バー

### 設定
- [ ] テーマ変更
- [ ] フォント変更
- [ ] 保存先変更

### ウィンドウ
- [ ] グローバルホットキー
- [ ] 位置記憶
- [ ] システムトレイ
```

---

## 9. マイグレーション計画

### 9.1 段階的移行

```
Phase 0: 準備 (1日)
├── 回帰テスト追加
├── 現行動作の記録
└── ベンチマーク取得

Phase 1: SQLite導入 (3-5日)
├── rusqlite 依存追加
├── SqliteIndex 実装
├── HybridRepository 実装
├── 既存テスト通過確認
└── 性能ベンチマーク

Phase 2: フロントエンド仮想化 (2-3日)
├── VirtualList コンポーネント
├── VirtualSidebar 置き換え
├── CommandPalette 最適化
└── UIテスト

Phase 3: ページネーションAPI (1-2日)
├── list_notes_paginated 追加
├── フロントエンドAPI追加
├── ストア更新
└── 結合テスト

Phase 4: バックリンク最適化 (1-2日)
├── BacklinkService 改修
├── メモリ使用量確認
└── 回帰テスト

Phase 5: エディタ最適化 (2-3日)
├── LivePreview 最適化
├── WikiLink 最適化
├── 設定変更の動的更新
└── 大ファイルテスト

Phase 6: 並列処理 (1-2日)
├── 並列ファイルスキャン
├── 並列インデックス構築
└── 最終ベンチマーク
```

### 9.2 ロールバック計画

```rust
// 機能フラグによる切り替え（開発中のみ）

// src-tauri/src/app_state.rs
pub struct AppState {
    // 新旧リポジトリを両方保持
    pub legacy_repository: Arc<FileNoteRepository>,
    pub hybrid_repository: Arc<HybridRepository>,

    // 機能フラグ
    pub use_sqlite_index: AtomicBool,
}

// コマンドで切り替え可能
#[tauri::command]
pub fn toggle_sqlite_index(state: State<AppState>, enabled: bool) {
    state.use_sqlite_index.store(enabled, Ordering::Release);
}
```

### 9.3 データマイグレーション

```rust
// 初回起動時の自動マイグレーション

impl HybridRepository {
    pub fn initialize(&self) -> Result<(), RepositoryError> {
        // インデックスDBが存在しない or バージョン不一致の場合
        if self.index.needs_rebuild()? {
            println!("[Migration] Building SQLite index...");

            // 進捗表示用にイベント発行
            self.event_bus.emit(DomainEvent::IndexRebuildStarted);

            let start = Instant::now();
            self.index.rebuild_parallel(&self.storage, &self.base_dir())?;

            println!("[Migration] Index built in {:?}", start.elapsed());
            self.event_bus.emit(DomainEvent::IndexRebuildCompleted);
        }

        Ok(())
    }
}
```

---

## 10. 期待される性能改善

### 10.1 目標メトリクス

| 操作 | 現在 (100件) | 現在 (100k件) | 目標 (100k件) |
|-----|-------------|--------------|--------------|
| `list_notes()` | <10ms | 2.5秒 | <50ms |
| `search_notes()` | <50ms | 1-3秒 | <100ms |
| `resolve_wiki_link()` | <10ms | 200ms | <10ms |
| `get_backlinks()` | <50ms | 500ms | <50ms |
| サイドバースクロール | 60fps | フリーズ | 60fps |
| エディタ入力遅延 | <16ms | <16ms | <16ms |
| メモリ使用量 | 50MB | 600MB+ | <100MB |
| 初回起動 | <1秒 | 10秒+ | <3秒 |

### 10.2 計測方法

```rust
// 開発時のベンチマーク計測

#[cfg(debug_assertions)]
fn measure<T, F: FnOnce() -> T>(name: &str, f: F) -> T {
    let start = std::time::Instant::now();
    let result = f();
    let elapsed = start.elapsed();
    println!("[PERF] {} took {:?}", name, elapsed);
    result
}

// 使用例
let items = measure("list_all", || repo.list_all())?;
```

### 10.3 成功基準

1. **機能等価性**: 全ての既存E2Eテストがパス
2. **API互換性**: 全てのTauriコマンドのシグネチャ維持
3. **性能目標**: 上記メトリクス達成
4. **安定性**: 100k件で24時間連続使用可能

---

## 付録

### A. 依存関係一覧

```toml
# 追加が必要な依存関係

[dependencies]
rusqlite = { version = "0.31", features = ["bundled"] }
blake3 = "1.5"

# 既存（変更なし）
rayon = "1.10"
```

### B. ファイル一覧（新規/変更）

```
src-tauri/src/
├── infrastructure/
│   ├── sqlite_index.rs       # NEW
│   ├── hybrid_repository.rs  # NEW
│   └── file_repository.rs    # MODIFY (internal use)
├── services/
│   └── backlink_service.rs   # MODIFY
├── commands/
│   └── note.rs               # MODIFY (add paginated)
└── tests/
    └── regression.rs         # NEW

src/lib/
├── components/
│   ├── VirtualList.svelte    # NEW
│   └── Sidebar.svelte        # MODIFY → VirtualSidebar
├── editor/extensions/
│   ├── livePreview.ts        # MODIFY
│   └── wikiLink.ts           # MODIFY
├── stores/
│   └── note.svelte.ts        # MODIFY
└── services/
    └── api.ts                # MODIFY (add paginated)
```

### C. 参考リンク

- [rusqlite documentation](https://docs.rs/rusqlite/)
- [SQLite FTS5](https://www.sqlite.org/fts5.html)
- [CodeMirror 6 Compartments](https://codemirror.net/docs/ref/#state.Compartment)
- [Svelte 5 Snippets](https://svelte.dev/docs/svelte/snippet)
- [Rayon parallel iterators](https://docs.rs/rayon/)

---

**文書バージョン**: 1.0
**作成日**: 2025-01-22
**対象アプリバージョン**: kaku 0.1.0
