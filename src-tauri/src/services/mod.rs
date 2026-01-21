pub mod note_service;
pub mod settings_service;
pub mod window_service;
pub mod search_service;

pub use note_service::NoteService;
pub use settings_service::SettingsService;
pub use window_service::{WindowService, ToggleResult};
pub use search_service::SearchService;
