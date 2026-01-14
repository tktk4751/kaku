use crate::domain::Note;
use crate::traits::NoteListItem;
use crate::AppState;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{Manager, State};

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

/// 新規メモを作成
#[tauri::command]
pub fn create_note(state: State<AppState>) -> Result<NoteDto, String> {
    state
        .note_service
        .create_note()
        .map(NoteDto::from)
        .map_err(|e| e.to_string())
}

/// メモを保存
#[tauri::command]
pub fn save_note(state: State<AppState>, uid: String, content: String) -> Result<(), String> {
    // 既存ノートをロード、なければ新規作成
    let mut note = match state.note_service.load_note(&uid) {
        Ok(n) => n,
        Err(_) => {
            // ノートが存在しない場合は新規作成（UIDを保持）
            crate::domain::Note::with_uid(uid)
        }
    };

    note.update_content(content);
    state
        .note_service
        .save_note(&note)
        .map_err(|e| e.to_string())
}

/// メモをロード
#[tauri::command]
pub fn load_note(state: State<AppState>, uid: String) -> Result<NoteDto, String> {
    state
        .note_service
        .load_note(&uid)
        .map(NoteDto::from)
        .map_err(|e| e.to_string())
}

/// メモを削除
#[tauri::command]
pub fn delete_note(state: State<AppState>, uid: String) -> Result<(), String> {
    state
        .note_service
        .delete_note(&uid)
        .map_err(|e| e.to_string())
}

/// 全メモ一覧を取得
#[tauri::command]
pub fn list_notes(state: State<AppState>) -> Result<Vec<NoteListItemDto>, String> {
    state
        .note_service
        .list_notes()
        .map(|items| items.into_iter().map(NoteListItemDto::from).collect())
        .map_err(|e| e.to_string())
}

/// 設定を取得
#[tauri::command]
pub fn get_settings(state: State<AppState>) -> crate::domain::Settings {
    state.settings_service.get()
}

/// 設定を更新
#[tauri::command]
pub fn update_settings(state: State<AppState>, settings: SettingsUpdateDto) -> Result<(), String> {
    state
        .settings_service
        .update(|s| {
            if let Some(theme) = settings.theme {
                s.theme = theme;
            }
            if let Some(theme_mode) = settings.theme_mode {
                s.theme_mode = theme_mode;
            }
            if let Some(font_family) = settings.font_family {
                s.editor.font_family = font_family;
            }
            if let Some(font_size) = settings.font_size {
                s.editor.font_size = font_size;
            }
            if let Some(line_height) = settings.line_height {
                s.editor.line_height = line_height;
            }
            if let Some(show_line_numbers) = settings.show_line_numbers {
                s.editor.show_line_numbers = show_line_numbers;
            }
            if let Some(autosave_enabled) = settings.autosave_enabled {
                s.autosave.enabled = autosave_enabled;
            }
            if let Some(autosave_delay) = settings.autosave_delay_ms {
                s.autosave.delay_ms = autosave_delay;
            }
            if let Some(restore_last) = settings.restore_last_note {
                s.restore_last_note = restore_last;
            }
            if let Some(storage_directory) = settings.storage_directory {
                s.storage_directory = storage_directory;
            }
            if let Some(shortcut) = settings.shortcut_new_note {
                s.shortcuts.new_note = shortcut;
            }
            if let Some(shortcut) = settings.shortcut_toggle_sidebar {
                s.shortcuts.toggle_sidebar = shortcut;
            }
            if let Some(shortcut) = settings.shortcut_open_settings {
                s.shortcuts.open_settings = shortcut;
            }
        })
        .map_err(|e| e.to_string())
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

/// ホットキーを更新
#[tauri::command]
pub fn update_hotkey(state: State<AppState>, hotkey: String) -> Result<(), String> {
    // Hyprland環境の場合、bindings.confを更新
    #[cfg(target_os = "linux")]
    {
        if crate::platform::hyprland::is_hyprland() {
            crate::platform::hyprland::update_hotkey_binding(&hotkey)?;
        }
    }

    // 設定ファイルに保存
    state
        .settings_service
        .update(|s| {
            s.hotkey = hotkey.clone();
        })
        .map_err(|e| e.to_string())?;

    println!("[Hotkey] Updated to: {}", hotkey);
    Ok(())
}

/// 現在のホットキーを取得（プラットフォーム対応）
#[tauri::command]
pub fn get_current_hotkey(state: State<AppState>) -> String {
    #[cfg(target_os = "linux")]
    {
        if crate::platform::hyprland::is_hyprland() {
            if let Some(hotkey) = crate::platform::hyprland::get_current_hotkey() {
                return hotkey;
            }
        }
    }

    // 設定ファイルから取得
    state.settings_service.get().hotkey
}

/// ウィンドウジオメトリを保存
#[tauri::command]
pub fn save_window_geometry(
    app: tauri::AppHandle,
    state: State<AppState>,
) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or("Window not found")?;

    let geometry = crate::platform::WindowManager::get_geometry(&window)
        .map_err(|e| e.to_string())?;

    state
        .settings_service
        .update_window_geometry(geometry)
        .map_err(|e| e.to_string())
}

/// アプリ終了前の保存処理（ウィンドウ非表示時に呼ばれる）
#[tauri::command]
pub async fn prepare_hide(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    uid: Option<String>,
    content: Option<String>,
) -> Result<(), String> {
    // コンテンツがあれば保存
    if let (Some(ref uid_str), Some(content)) = (&uid, content) {
        let mut note = state
            .note_service
            .load_note(uid_str)
            .map_err(|e| e.to_string())?;

        if note.content != content {
            note.update_content(content);
            state
                .note_service
                .save_note(&note)
                .map_err(|e| e.to_string())?;
        }
    }

    // 最後に開いたノートを記録
    let _ = state.settings_service.update_last_note_uid(uid);

    // ジオメトリを保存
    if let Some(window) = app.get_webview_window("main") {
        if let Ok(geometry) = crate::platform::WindowManager::get_geometry(&window) {
            let _ = state.settings_service.update_window_geometry(geometry);
        }
    }

    Ok(())
}

/// 最後に開いたノートのUIDを更新
#[tauri::command]
pub fn set_last_note_uid(state: State<AppState>, uid: Option<String>) -> Result<(), String> {
    state
        .settings_service
        .update_last_note_uid(uid)
        .map_err(|e| e.to_string())
}

/// アプリケーションを終了
#[tauri::command]
pub fn quit_app(app: tauri::AppHandle) {
    app.exit(0);
}

/// ウィンドウを非表示にする
#[tauri::command]
pub fn hide_window(app: tauri::AppHandle, state: State<AppState>) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        // ジオメトリを保存
        #[cfg(target_os = "linux")]
        {
            if crate::platform::hyprland::is_hyprland() {
                if let Some((x, y)) = crate::platform::hyprland::get_window_position("kaku") {
                    // オフスクリーン位置は保存しない
                    if x >= -5000 && y >= -5000 {
                        let mut geometry = crate::platform::WindowManager::get_geometry(&window)
                            .unwrap_or_default();
                        geometry.x = x;
                        geometry.y = y;
                        let _ = state.settings_service.update_window_geometry(geometry);
                    }
                }
                // Hyprlandではオフスクリーンに移動
                crate::platform::hyprland::move_offscreen("kaku");
                crate::platform::mark_window_hidden();
                return Ok(());
            }
        }

        // 通常のhide
        if let Ok(geometry) = crate::platform::WindowManager::get_geometry(&window) {
            let _ = state.settings_service.update_window_geometry(geometry);
        }
        let _ = window.hide();
        crate::platform::mark_window_hidden();
    }
    Ok(())
}

/// ウィンドウを最大化/元に戻す
#[tauri::command]
pub fn toggle_maximize(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        if window.is_maximized().unwrap_or(false) {
            window.unmaximize().map_err(|e| e.to_string())?;
        } else {
            window.maximize().map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}
