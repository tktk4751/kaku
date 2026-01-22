// アプリケーション状態
//
// SOLID: Dependency Injection Container
// 全てのサービスとその依存関係をここで構築・管理する

use crate::infrastructure::{
    EventBusImpl, FileNoteRepository, FileSettingsRepository, FileStorage, HeadingFilenameStrategy,
};
use crate::services::{BacklinkService, NoteService, SearchService, SettingsService};
use std::sync::Arc;

/// アプリケーション状態（Dependency Injection Container）
pub struct AppState {
    pub note_service: NoteService,
    pub search_service: SearchService,
    pub backlink_service: Arc<BacklinkService>,
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
        let note_service = NoteService::new(note_repository.clone(), event_bus.clone());

        // Search Service
        let search_service = SearchService::new(note_repository.clone());

        // Backlink Service
        let backlink_service = Arc::new(BacklinkService::new(note_repository));

        // Build initial backlink index
        if let Err(e) = backlink_service.rebuild_index() {
            eprintln!("[AppState] Failed to build backlink index: {}", e);
        }

        Self {
            note_service,
            search_service,
            backlink_service,
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
