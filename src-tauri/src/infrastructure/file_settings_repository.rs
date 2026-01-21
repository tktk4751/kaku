// ファイルベースの設定リポジトリ実装
//
// SOLID: Single Responsibility Principle
// このモジュールは設定の永続化のみを担当する

use crate::domain::{Settings, SettingsError};
use crate::traits::SettingsRepository;
use std::path::PathBuf;

/// ファイルシステムを使用した設定リポジトリ
pub struct FileSettingsRepository {
    config_path: PathBuf,
}

impl FileSettingsRepository {
    pub fn new() -> Self {
        Self {
            config_path: Settings::config_path(),
        }
    }

    /// テスト用: カスタムパスで作成
    #[cfg(test)]
    pub fn with_path(config_path: PathBuf) -> Self {
        Self { config_path }
    }
}

impl Default for FileSettingsRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl SettingsRepository for FileSettingsRepository {
    fn load(&self) -> Result<Settings, SettingsError> {
        Settings::load_from_file(&self.config_path)
    }

    fn save(&self, settings: &Settings) -> Result<(), SettingsError> {
        settings.save_to_file(&self.config_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_save_and_load() {
        let temp_file = NamedTempFile::new().unwrap();
        let repo = FileSettingsRepository::with_path(temp_file.path().to_path_buf());

        let settings = Settings::default();
        repo.save(&settings).unwrap();

        let loaded = repo.load().unwrap();
        assert_eq!(settings, loaded);
    }

    #[test]
    fn test_load_nonexistent_returns_error() {
        let repo = FileSettingsRepository::with_path(PathBuf::from("/nonexistent/path/config.toml"));
        assert!(repo.load().is_err());
    }
}
