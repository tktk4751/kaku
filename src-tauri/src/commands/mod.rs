// コマンドモジュール - Tauri IPC ハンドラ
//
// SOLID: Single Responsibility
// 各サブモジュールは特定の関心事のコマンドのみを担当

pub mod note;
pub mod settings;
pub mod window;
pub mod hotkey;

// コマンド関数を re-export
pub use note::{create_note, save_note, load_note, delete_note, list_notes};
pub use settings::{get_settings, update_settings};
pub use window::{save_window_geometry, prepare_hide, set_last_note_uid, quit_app, hide_window, toggle_maximize};
pub use hotkey::{update_hotkey, get_current_hotkey};

// ===== DTO 定義（共有）=====

use crate::domain::Note;
use crate::traits::NoteListItem;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// フロントエンド用のノートDTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteDto {
    pub uid: String,
    pub content: String,
    pub created_at: String,
    pub updated_at: String,
    pub is_dirty: bool,
}

impl From<Note> for NoteDto {
    fn from(note: Note) -> Self {
        Self {
            uid: note.metadata.uid,
            content: note.content,
            created_at: note.metadata.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
            updated_at: note.metadata.updated_at.format("%Y-%m-%d %H:%M:%S").to_string(),
            is_dirty: note.is_dirty,
        }
    }
}

/// フロントエンド用のノート一覧アイテムDTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteListItemDto {
    pub uid: String,
    pub title: String,
    pub updated_at: String,
}

impl From<NoteListItem> for NoteListItemDto {
    fn from(item: NoteListItem) -> Self {
        Self {
            uid: item.uid,
            title: item.title,
            updated_at: item.updated_at.format("%Y-%m-%d %H:%M:%S").to_string(),
        }
    }
}

/// 設定更新DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingsUpdateDto {
    pub theme: Option<crate::domain::ThemeName>,
    pub theme_mode: Option<crate::domain::ThemeMode>,
    pub font_family: Option<String>,
    pub font_size: Option<u32>,
    pub line_height: Option<f32>,
    pub show_line_numbers: Option<bool>,
    pub autosave_enabled: Option<bool>,
    pub autosave_delay_ms: Option<u64>,
    pub restore_last_note: Option<bool>,
    pub storage_directory: Option<PathBuf>,
    pub hotkey: Option<String>,
    pub shortcut_new_note: Option<String>,
    pub shortcut_toggle_sidebar: Option<String>,
    pub shortcut_open_settings: Option<String>,
}
