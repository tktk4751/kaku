use crate::domain::{DomainEvent, Settings, SettingsError, WindowGeometry};
use crate::traits::EventBus;
use parking_lot::RwLock;
use std::path::PathBuf;
use std::sync::Arc;

/// 設定サービス
pub struct SettingsService {
    settings: RwLock<Settings>,
    config_path: PathBuf,
    event_bus: Arc<dyn EventBus>,
}

impl SettingsService {
    pub fn new(event_bus: Arc<dyn EventBus>) -> Self {
        let config_path = Settings::config_path();

        // 設定をロード（存在しなければデフォルト）
        let settings = Settings::load_from_file(&config_path).unwrap_or_default();

        Self {
            settings: RwLock::new(settings),
            config_path,
            event_bus,
        }
    }

    /// 設定を取得
    pub fn get(&self) -> Settings {
        self.settings.read().clone()
    }

    /// 設定を更新
    pub fn update<F>(&self, f: F) -> Result<(), SettingsError>
    where
        F: FnOnce(&mut Settings),
    {
        {
            let mut settings = self.settings.write();
            f(&mut settings);
            settings.save_to_file(&self.config_path)?;
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
