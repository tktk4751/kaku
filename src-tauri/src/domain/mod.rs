pub mod note;
pub mod settings;
pub mod events;
pub mod search;

pub use note::{Note, NoteMetadata, NoteParseError};
pub use settings::{Settings, SettingsError, WindowGeometry, EditorSettings, ThemeName, ThemeMode, AutosaveSettings, ShortcutSettings};
pub use events::DomainEvent;
pub use search::{SearchResult, MatchRange, ContentPreview, SearchError};
