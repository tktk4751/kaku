// アプリケーション状態
//
// SOLID: Dependency Injection Container
// 全てのサービスとその依存関係をここで構築・管理する

use crate::infrastructure::{
    EventBusImpl, FileNoteRepository, FileSettingsRepository, FileStorage, HeadingFilenameStrategy,
};
use crate::services::{NoteService, SettingsService};
use std::sync::Arc;

/// アプリケーション状態（Dependency Injection Container）
pub struct AppState {
    pub note_service: NoteService,
    pub settings_service: Arc<SettingsService>,
    pub event_bus: Arc<EventBusImpl>,
}

impl AppState {
    pub fn new() -> Self {
        // EventBus
        let event_bus = Arc::new(EventBusImpl::new());

        // Settings Repository & Service
        // SOLID: Dependency Inversion - Repository を注入
        let settings_repository = Arc::new(FileSettingsRepository::new());
        let settings_service = Arc::new(SettingsService::new(
            settings_repository,
            event_bus.clone(),
        ));

        // Storage & Repository
        let storage = Arc::new(FileStorage::new());
        let filename_strategy = Arc::new(HeadingFilenameStrategy::new());
        let note_repository = Arc::new(FileNoteRepository::new(
            storage,
            filename_strategy,
            settings_service.clone(),
        ));

        // Note Service
        let note_service = NoteService::new(note_repository, event_bus.clone());

        Self {
            note_service,
            settings_service,
            event_bus,
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
