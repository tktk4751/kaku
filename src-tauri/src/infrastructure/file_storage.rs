use crate::traits::{Storage, StorageError};
use std::fs;
use std::path::{Path, PathBuf};

/// ファイルシステムベースのストレージ実装
pub struct FileStorage;

impl FileStorage {
    pub fn new() -> Self {
        Self
    }
}

impl Default for FileStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl Storage for FileStorage {
    fn save_atomic(&self, path: &Path, content: &str) -> Result<(), StorageError> {
        // 親ディレクトリを作成
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|_| {
                StorageError::CreateDirFailed(parent.to_path_buf())
            })?;
        }

        // 一時ファイルに書き込み
        let temp_path = path.with_extension("md.tmp");
        fs::write(&temp_path, content)?;

        // アトミックにリネーム
        fs::rename(&temp_path, path)?;

        Ok(())
    }

    fn load(&self, path: &Path) -> Result<String, StorageError> {
        if !path.exists() {
            return Err(StorageError::NotFound(path.to_path_buf()));
        }
        fs::read_to_string(path).map_err(StorageError::from)
    }

    fn delete(&self, path: &Path) -> Result<(), StorageError> {
        if !path.exists() {
            return Err(StorageError::NotFound(path.to_path_buf()));
        }
        fs::remove_file(path).map_err(StorageError::from)
    }

    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }

    fn list_files(&self, dir: &Path, extension: &str) -> Result<Vec<PathBuf>, StorageError> {
        if !dir.exists() {
            return Ok(Vec::new());
        }

        let entries = fs::read_dir(dir)?;
        let mut files = Vec::new();

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == extension {
                        files.push(path);
                    }
                }
            }
        }

        Ok(files)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let storage = FileStorage::new();
        let path = temp_dir.path().join("test.md");

        let content = "# Test\n\nHello, world!";
        storage.save_atomic(&path, content).unwrap();

        let loaded = storage.load(&path).unwrap();
        assert_eq!(content, loaded);
    }

    #[test]
    fn test_atomic_save_creates_parent_dirs() {
        let temp_dir = TempDir::new().unwrap();
        let storage = FileStorage::new();
        let path = temp_dir.path().join("subdir").join("test.md");

        storage.save_atomic(&path, "content").unwrap();
        assert!(path.exists());
    }

    #[test]
    fn test_list_files() {
        let temp_dir = TempDir::new().unwrap();
        let storage = FileStorage::new();

        // テストファイル作成
        fs::write(temp_dir.path().join("a.md"), "content").unwrap();
        fs::write(temp_dir.path().join("b.md"), "content").unwrap();
        fs::write(temp_dir.path().join("c.txt"), "content").unwrap();

        let files = storage.list_files(temp_dir.path(), "md").unwrap();
        assert_eq!(files.len(), 2);
    }
}
