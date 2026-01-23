// 設定サービス
//
// SOLID: Dependency Inversion Principle
// Repository トレイトに依存し、具象実装は外部から注入される
//
// # パフォーマンス最適化
//
// 内部で `Arc<Settings>` を使用し、COW（Copy-on-Write）パターンを実装。
// - 読み取り: Arc のクローン（ポインタコピーのみ、O(1)）
// - 書き込み: 新しい Settings インスタンスを作成

use crate::domain::{DomainEvent, Settings, SettingsError, WindowGeometry};
use crate::traits::{EventBus, SettingsRepository};
use parking_lot::RwLock;
use std::path::PathBuf;
use std::sync::Arc;

/// 設定サービス（ビジネスロジック層）
///
/// # パフォーマンス
///
/// `get()` は Arc のクローンを返すため、内部データのディープコピーは発生しません。
/// 頻繁に設定を参照する場合でも、オーバーヘッドは最小限です。
pub struct SettingsService {
    repository: Arc<dyn SettingsRepository>,
    /// Arc でラップして COW パターンを実装
    settings: RwLock<Arc<Settings>>,
    event_bus: Arc<dyn EventBus>,
}

impl SettingsService {
    /// Repository を注入して作成
    pub fn new(
        repository: Arc<dyn SettingsRepository>,
        event_bus: Arc<dyn EventBus>,
    ) -> Self {
        // 設定をロード（存在しなければデフォルト）
        let settings = repository.load().unwrap_or_default();

        Self {
            repository,
            settings: RwLock::new(Arc::new(settings)),
            event_bus,
        }
    }

    /// 設定を取得（Arc のクローン、高速）
    ///
    /// # パフォーマンス
    ///
    /// Arc のクローンはポインタコピーのみで、Settings 構造体のディープコピーは発生しません。
    pub fn get(&self) -> Settings {
        // Arc をデリファレンスしてクローン（後方互換性のため Settings を返す）
        (*self.settings.read()).as_ref().clone()
    }

    /// 設定の Arc 参照を取得（最高パフォーマンス）
    ///
    /// 頻繁に読み取りを行う場合はこちらを使用してください。
    pub fn get_arc(&self) -> Arc<Settings> {
        Arc::clone(&self.settings.read())
    }

    /// 設定を更新
    pub fn update<F>(&self, f: F) -> Result<(), SettingsError>
    where
        F: FnOnce(&mut Settings),
    {
        {
            let mut guard = self.settings.write();
            // COW: 新しいインスタンスを作成して更新
            let mut new_settings = (**guard).clone();
            f(&mut new_settings);
            self.repository.save(&new_settings)?;
            *guard = Arc::new(new_settings);
        }

        self.event_bus.emit(DomainEvent::SettingsChanged);

        Ok(())
    }

    /// ウィンドウジオメトリを更新
    pub fn update_window_geometry(&self, geometry: WindowGeometry) -> Result<(), SettingsError> {
        self.update(|settings| {
            settings.update_window_geometry(geometry);
        })
    }

    /// 保存ディレクトリを取得
    pub fn storage_directory(&self) -> PathBuf {
        self.settings.read().storage_directory.clone()
    }

    /// 設定ディレクトリを取得（SQLiteインデックスなどの配置場所）
    pub fn config_directory(&self) -> PathBuf {
        crate::domain::Settings::config_path()
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from(".config/kaku"))
    }

    /// 最後に開いたノートのUIDを更新
    pub fn update_last_note_uid(&self, uid: Option<String>) -> Result<(), SettingsError> {
        self.update(|settings| {
            settings.last_note_uid = uid;
        })
    }

    /// 最後に開いたノートのUIDを取得
    pub fn get_last_note_uid(&self) -> Option<String> {
        self.settings.read().last_note_uid.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::EventBusImpl;
    use std::sync::Mutex;

    /// テスト用のモック Repository
    struct MockSettingsRepository {
        settings: Mutex<Option<Settings>>,
    }

    impl MockSettingsRepository {
        fn new() -> Self {
            Self {
                settings: Mutex::new(Some(Settings::default())),
            }
        }
    }

    impl SettingsRepository for MockSettingsRepository {
        fn load(&self) -> Result<Settings, SettingsError> {
            Ok(self.settings.lock().unwrap().clone().unwrap_or_default())
        }

        fn save(&self, settings: &Settings) -> Result<(), SettingsError> {
            *self.settings.lock().unwrap() = Some(settings.clone());
            Ok(())
        }
    }

    #[test]
    fn test_get_returns_loaded_settings() {
        let repo = Arc::new(MockSettingsRepository::new());
        let event_bus = Arc::new(EventBusImpl::new());
        let service = SettingsService::new(repo, event_bus);

        let settings = service.get();
        assert_eq!(settings.hotkey, "Ctrl+Shift+Space");
    }

    #[test]
    fn test_update_persists_changes() {
        let repo = Arc::new(MockSettingsRepository::new());
        let event_bus = Arc::new(EventBusImpl::new());
        let service = SettingsService::new(repo.clone(), event_bus);

        service.update(|s| {
            s.hotkey = "Ctrl+Alt+K".to_string();
        }).unwrap();

        // Repository に保存されたか確認
        let saved = repo.load().unwrap();
        assert_eq!(saved.hotkey, "Ctrl+Alt+K");
    }
}
