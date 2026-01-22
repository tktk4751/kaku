//! ファイルベースのノートリポジトリ
//!
//! # キャッシュ戦略と安全性
//!
//! このリポジトリは2種類のキャッシュを持ちます：
//!
//! 1. **パスキャッシュ** (`path_cache`): UID → ファイルパス
//! 2. **リストキャッシュ** (`list_cache`): ノート一覧のキャッシュ
//!
//! ## キャッシュの特性
//!
//! - **書き込み時更新**: `save()` 時に両キャッシュを更新
//! - **読み込み時ポピュレート**: `load()` 時にキャッシュミスがあればディレクトリをスキャン
//! - **削除時無効化**: `delete()` 時にキャッシュからエントリを削除
//!
//! ## リストキャッシュの最適化
//!
//! `list_all()` は頻繁に呼ばれるため、以下の最適化を行います：
//!
//! - キャッシュが有効な場合（dirty = false）、ファイルを再読み込みせずにキャッシュを返す
//! - `save()` や `delete()` 時にキャッシュを dirty としてマーク
//! - dirty 状態の場合のみファイルを再スキャン
//!
//! ## 外部変更の検出
//!
//! **重要**: 外部ツール（ファイラー、エディタ等）でファイルが変更された場合、
//! キャッシュは自動更新されません。以下の制限があります：
//!
//! - 外部で追加されたファイル: 次回の `list_all()` または `load()` でキャッシュに追加
//! - 外部で削除されたファイル: `load()` 時にエラー、その後キャッシュから削除
//! - 外部でリネームされたファイル: 古いパスでアクセス時にエラー
//!
//! ## スレッドセーフティ
//!
//! - キャッシュは `RwLock` で保護
//! - 複数スレッドからの同時アクセスは安全
//! - ただし、同一ノートへの同時書き込みは最後の書き込みが優先（last-write-wins）
//!
//! ## 推奨事項
//!
//! - ノートファイルの外部編集は避ける
//! - 大規模コレクション（1000+ノート）ではキャッシュウォームアップを検討

use crate::domain::Note;
use crate::services::SettingsService;
use crate::traits::{FilenameStrategy, NoteRepository, NoteListItem, RepositoryError, Storage};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// 保存ディレクトリの取得方法
enum BaseDirSource {
    /// 設定サービスから動的に取得
    Settings(Arc<SettingsService>),
    /// 固定パス（テスト用）
    #[cfg(test)]
    Fixed(PathBuf),
}

/// ファイルベースのノートリポジトリ実装
///
/// # キャッシュ
///
/// - `path_cache`: UID からファイルパスへのマッピング（ロード時のスキャン回避）
/// - `list_cache`: ノート一覧のキャッシュ（list_all()の高速化）
///
/// キャッシュの一貫性はこのリポジトリ経由の操作でのみ保証されます。
/// 外部でファイルが変更された場合、一貫性は保証されません。
pub struct FileNoteRepository {
    storage: Arc<dyn Storage>,
    filename_strategy: Arc<dyn FilenameStrategy>,
    base_dir_source: BaseDirSource,
    /// UID → ファイルパスのキャッシュ
    ///
    /// 注意: 外部でファイルが変更された場合、このキャッシュは古くなる可能性があります。
    path_cache: RwLock<HashMap<String, PathBuf>>,
    /// ノート一覧キャッシュ（list_all()の高速化）
    ///
    /// dirty フラグが true の場合、次回の list_all() で再構築されます。
    list_cache: RwLock<Vec<NoteListItem>>,
    /// リストキャッシュが無効（再構築が必要）かどうか
    list_cache_dirty: AtomicBool,
}

impl FileNoteRepository {
    pub fn new(
        storage: Arc<dyn Storage>,
        filename_strategy: Arc<dyn FilenameStrategy>,
        settings_service: Arc<SettingsService>,
    ) -> Self {
        Self {
            storage,
            filename_strategy,
            base_dir_source: BaseDirSource::Settings(settings_service),
            path_cache: RwLock::new(HashMap::new()),
            list_cache: RwLock::new(Vec::new()),
            list_cache_dirty: AtomicBool::new(true), // 初回は再構築が必要
        }
    }

    /// テスト用: 固定パスで作成
    #[cfg(test)]
    pub fn with_fixed_path(
        storage: Arc<dyn Storage>,
        filename_strategy: Arc<dyn FilenameStrategy>,
        base_dir: PathBuf,
    ) -> Self {
        Self {
            storage,
            filename_strategy,
            base_dir_source: BaseDirSource::Fixed(base_dir),
            path_cache: RwLock::new(HashMap::new()),
            list_cache: RwLock::new(Vec::new()),
            list_cache_dirty: AtomicBool::new(true),
        }
    }

    /// 現在の保存ディレクトリを取得（設定から動的に）
    fn base_dir(&self) -> PathBuf {
        match &self.base_dir_source {
            BaseDirSource::Settings(settings) => settings.storage_directory(),
            #[cfg(test)]
            BaseDirSource::Fixed(path) => path.clone(),
        }
    }

    /// キャッシュを再構築
    pub fn rebuild_cache(&self) -> Result<(), RepositoryError> {
        let files = self.storage.list_files(&self.base_dir(), "md")?;
        let mut cache = self.path_cache.write();
        cache.clear();

        for path in files {
            if let Ok(content) = self.storage.load(&path) {
                if let Ok(note) = Note::from_file_content(&content) {
                    cache.insert(note.metadata.uid, path);
                }
            }
        }

        // リストキャッシュも無効化
        self.invalidate_list_cache();

        Ok(())
    }

    /// リストキャッシュを無効化（次回のlist_all()で再構築される）
    fn invalidate_list_cache(&self) {
        self.list_cache_dirty.store(true, Ordering::Release);
    }

    /// リストキャッシュを更新
    fn update_list_cache(&self, items: Vec<NoteListItem>) {
        let mut cache = self.list_cache.write();
        *cache = items;
        self.list_cache_dirty.store(false, Ordering::Release);
    }

    /// リストキャッシュ内の特定アイテムを更新（save時）
    fn update_list_cache_item(&self, note: &Note, path: &Path) {
        let title = note
            .extract_heading()
            .unwrap_or_else(|| note.metadata.uid.clone());

        let new_item = NoteListItem {
            uid: note.metadata.uid.clone(),
            title,
            path: path.to_path_buf(),
            updated_at: note.metadata.updated_at,
        };

        let mut cache = self.list_cache.write();

        // 既存のエントリを更新または追加
        if let Some(existing) = cache.iter_mut().find(|item| item.uid == note.metadata.uid) {
            *existing = new_item;
        } else {
            cache.push(new_item);
        }

        // 更新日時でソート（新しい順）
        cache.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    }

    /// リストキャッシュから特定アイテムを削除
    fn remove_from_list_cache(&self, uid: &str) {
        let mut cache = self.list_cache.write();
        cache.retain(|item| item.uid != uid);
    }

    /// 既存ファイルパスの一覧を取得
    fn get_existing_files(&self) -> Vec<PathBuf> {
        self.storage
            .list_files(&self.base_dir(), "md")
            .unwrap_or_default()
    }
}

impl NoteRepository for FileNoteRepository {
    fn save(&self, note: &Note) -> Result<PathBuf, RepositoryError> {
        // 既存のパスがあればそれを使用、なければ新規生成
        let path = {
            let cache = self.path_cache.read();
            cache.get(&note.metadata.uid).cloned()
        };

        let path = match path {
            Some(existing_path) => existing_path,
            None => {
                let existing_files = self.get_existing_files();
                let refs: Vec<&Path> = existing_files.iter().map(|p| p.as_path()).collect();
                let filename = self.filename_strategy.generate(note, &refs);
                self.base_dir().join(format!("{}.md", filename))
            }
        };

        // ファイルに保存
        let content = note.to_file_content();
        self.storage.save_atomic(&path, &content)?;

        // パスキャッシュを更新
        {
            let mut cache = self.path_cache.write();
            cache.insert(note.metadata.uid.clone(), path.clone());
        }

        // リストキャッシュを増分更新（dirty でない場合のみ）
        // dirty の場合は次回の list_all() で再構築されるので不要
        if !self.list_cache_dirty.load(Ordering::Acquire) {
            self.update_list_cache_item(note, &path);
        }

        Ok(path)
    }

    fn load(&self, uid: &str) -> Result<Note, RepositoryError> {
        // Try cache first
        let path = match self.get_path(uid) {
            Some(p) => p,
            None => {
                // Cache miss: search all files for matching UID
                // 一時的なHashMapに収集し、最後に一度だけキャッシュを更新（競合状態を回避）
                let files = self.storage.list_files(&self.base_dir(), "md")?;
                let mut found_path = None;
                let mut discovered_entries: Vec<(String, PathBuf)> = Vec::new();

                for file_path in files {
                    if let Ok(content) = self.storage.load(&file_path) {
                        if let Ok(note) = Note::from_file_content(&content) {
                            discovered_entries.push((note.metadata.uid.clone(), file_path.clone()));

                            if note.metadata.uid == uid {
                                found_path = Some(file_path);
                                break;
                            }
                        }
                    }
                }

                // バッチでキャッシュを更新（単一のロック取得）
                if !discovered_entries.is_empty() {
                    let mut cache = self.path_cache.write();
                    for (discovered_uid, discovered_path) in discovered_entries {
                        cache.insert(discovered_uid, discovered_path);
                    }
                }

                found_path.ok_or_else(|| RepositoryError::not_found(uid))?
            }
        };

        let content = self.storage.load(&path)?;
        Note::from_file_content(&content).map_err(|_| RepositoryError::not_found(uid))
    }

    fn delete(&self, uid: &str) -> Result<(), RepositoryError> {
        let path = self
            .get_path(uid)
            .ok_or_else(|| RepositoryError::not_found(uid))?;

        self.storage.delete(&path)?;

        // パスキャッシュから削除
        {
            let mut cache = self.path_cache.write();
            cache.remove(uid);
        }

        // リストキャッシュから削除（dirty でない場合のみ）
        if !self.list_cache_dirty.load(Ordering::Acquire) {
            self.remove_from_list_cache(uid);
        }

        Ok(())
    }

    fn list_all(&self) -> Result<Vec<NoteListItem>, RepositoryError> {
        // キャッシュが有効な場合はキャッシュを返す
        if !self.list_cache_dirty.load(Ordering::Acquire) {
            let cache = self.list_cache.read();
            if !cache.is_empty() {
                return Ok(cache.clone());
            }
        }

        // キャッシュが無効または空の場合はファイルをスキャン
        let files = self.storage.list_files(&self.base_dir(), "md")?;
        let mut items = Vec::new();
        let mut cache_updates: Vec<(String, PathBuf)> = Vec::new();

        for path in files {
            if let Ok(content) = self.storage.load(&path) {
                if let Ok(note) = Note::from_file_content(&content) {
                    let title = note
                        .extract_heading()
                        .unwrap_or_else(|| note.metadata.uid.clone());

                    items.push(NoteListItem {
                        uid: note.metadata.uid.clone(),
                        title,
                        path: path.clone(),
                        updated_at: note.metadata.updated_at,
                    });

                    // Collect cache updates to batch
                    cache_updates.push((note.metadata.uid, path));
                }
            }
        }

        // Batch update path cache (single lock acquisition)
        if !cache_updates.is_empty() {
            let mut cache = self.path_cache.write();
            for (uid, path) in cache_updates {
                cache.insert(uid, path);
            }
        }

        // 更新日時でソート（新しい順）
        items.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

        // リストキャッシュを更新
        self.update_list_cache(items.clone());

        Ok(items)
    }

    fn get_path(&self, uid: &str) -> Option<PathBuf> {
        let cache = self.path_cache.read();
        cache.get(uid).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::{FileStorage, HeadingFilenameStrategy};
    use tempfile::TempDir;

    fn create_test_repo(temp_dir: &TempDir) -> FileNoteRepository {
        FileNoteRepository::with_fixed_path(
            Arc::new(FileStorage::new()),
            Arc::new(HeadingFilenameStrategy::new()),
            temp_dir.path().to_path_buf(),
        )
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
    fn test_list_cache_optimization() {
        let temp_dir = TempDir::new().unwrap();
        let repo = create_test_repo(&temp_dir);

        // 初期状態ではキャッシュはdirty
        assert!(repo.list_cache_dirty.load(Ordering::Acquire));

        // 最初のlist_all()でキャッシュが構築される
        let mut note1 = Note::new();
        note1.content = "# メモ 1\n\n本文".to_string();
        repo.save(&note1).unwrap();

        let items1 = repo.list_all().unwrap();
        assert_eq!(items1.len(), 1);

        // キャッシュが有効になっている
        assert!(!repo.list_cache_dirty.load(Ordering::Acquire));

        // 2回目のlist_all()はキャッシュから返される（同じ結果）
        let items2 = repo.list_all().unwrap();
        assert_eq!(items1.len(), items2.len());
        assert_eq!(items1[0].uid, items2[0].uid);

        // 新しいノートを保存するとキャッシュが増分更新される
        let mut note2 = Note::new();
        note2.content = "# メモ 2\n\n本文".to_string();
        repo.save(&note2).unwrap();

        // キャッシュはまだ有効（増分更新された）
        assert!(!repo.list_cache_dirty.load(Ordering::Acquire));

        // list_all()で2つのノートが返される
        let items3 = repo.list_all().unwrap();
        assert_eq!(items3.len(), 2);

        // 削除するとキャッシュから削除される
        repo.delete(&note1.metadata.uid).unwrap();

        // キャッシュはまだ有効
        assert!(!repo.list_cache_dirty.load(Ordering::Acquire));

        // list_all()で1つのノートが返される
        let items4 = repo.list_all().unwrap();
        assert_eq!(items4.len(), 1);
        assert_eq!(items4[0].uid, note2.metadata.uid);
    }
}
