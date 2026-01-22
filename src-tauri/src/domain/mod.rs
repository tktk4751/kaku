pub mod note;
pub mod settings;
pub mod events;
pub mod search;
pub mod backlink;

pub use note::{Note, NoteMetadata, NoteParseError};
pub use settings::{Settings, SettingsError, WindowGeometry, EditorSettings, ThemeName, ThemeMode, AutosaveSettings, ShortcutSettings};
pub use events::DomainEvent;
pub use search::{SearchResult, MatchRange, ContentPreview, SearchError};
pub use backlink::{BacklinkInfo, ExtractedLink, extract_wiki_links, extract_context};
