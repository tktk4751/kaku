use crate::domain::Note;
use crate::services::SettingsService;
use crate::traits::{FilenameStrategy, NoteRepository, NoteListItem, RepositoryError, Storage};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// 保存ディレクトリの取得方法
enum BaseDirSource {
    /// 設定サービスから動的に取得
    Settings(Arc<SettingsService>),
    /// 固定パス（テスト用）
    #[cfg(test)]
    Fixed(PathBuf),
}

/// ファイルベースのノートリポジトリ実装
pub struct FileNoteRepository {
    storage: Arc<dyn Storage>,
    filename_strategy: Arc<dyn FilenameStrategy>,
    base_dir_source: BaseDirSource,
    /// UID → ファイルパスのキャッシュ
    path_cache: RwLock<HashMap<String, PathBuf>>,
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
                    cache.insert(note.metadata.uid.clone(), path);
                }
            }
        }

        Ok(())
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

        // キャッシュを更新
        {
            let mut cache = self.path_cache.write();
            cache.insert(note.metadata.uid.clone(), path.clone());
        }

        Ok(path)
    }

    fn load(&self, uid: &str) -> Result<Note, RepositoryError> {
        // Try cache first
        let path = match self.get_path(uid) {
            Some(p) => p,
            None => {
                // Cache miss: search all files for matching UID
                let files = self.storage.list_files(&self.base_dir(), "md")?;
                let mut found_path = None;

                for file_path in files {
                    if let Ok(content) = self.storage.load(&file_path) {
                        if let Ok(note) = Note::from_file_content(&content) {
                            // Update cache while searching
                            let mut cache = self.path_cache.write();
                            cache.insert(note.metadata.uid.clone(), file_path.clone());

                            if note.metadata.uid == uid {
                                found_path = Some(file_path);
                                break;
                            }
                        }
                    }
                }

                found_path.ok_or_else(|| RepositoryError::NotFound(uid.to_string()))?
            }
        };

        let content = self.storage.load(&path)?;
        Note::from_file_content(&content).map_err(|e| RepositoryError::Parse(e.to_string()))
    }

    fn delete(&self, uid: &str) -> Result<(), RepositoryError> {
        let path = self
            .get_path(uid)
            .ok_or_else(|| RepositoryError::NotFound(uid.to_string()))?;

        self.storage.delete(&path)?;

        // キャッシュから削除
        {
            let mut cache = self.path_cache.write();
            cache.remove(uid);
        }

        Ok(())
    }

    fn list_all(&self) -> Result<Vec<NoteListItem>, RepositoryError> {
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

        // Batch update cache (single lock acquisition)
        if !cache_updates.is_empty() {
            let mut cache = self.path_cache.write();
            for (uid, path) in cache_updates {
                cache.insert(uid, path);
            }
        }

        // 更新日時でソート（新しい順）
        items.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

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
}
