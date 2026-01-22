// 設定関連コマンド
use super::SettingsUpdateDto;
use crate::AppState;
use std::path::Path;
use tauri::State;

/// 設定を取得
#[tauri::command]
pub fn get_settings(state: State<AppState>) -> crate::domain::Settings {
    state.settings_service.get()
}

/// ストレージディレクトリのパスを検証
///
/// # 検証項目
///
/// - パスが絶対パスであること
/// - パスに不正な文字が含まれていないこと
/// - ディレクトリが存在し書き込み可能、または作成可能であること
fn validate_storage_directory(path: &Path) -> Result<(), String> {
    // 絶対パスチェック
    if !path.is_absolute() {
        return Err("Storage directory must be an absolute path".to_string());
    }

    // パスを正規化して危険なコンポーネントをチェック
    // 注: canonicalize は存在しないパスでは失敗するため、コンポーネントを直接検査
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                // ".." を含むパスは拒否（パストラバーサル防止）
                return Err("Storage directory path cannot contain '..'".to_string());
            }
            std::path::Component::Normal(s) => {
                // 不正な文字をチェック（制御文字等）
                if let Some(s) = s.to_str() {
                    if s.chars().any(|c| c.is_control()) {
                        return Err("Storage directory path contains invalid characters".to_string());
                    }
                }
            }
            _ => {}
        }
    }

    // ディレクトリの存在と書き込み可能性をチェック
    if path.exists() {
        if !path.is_dir() {
            return Err("Storage path exists but is not a directory".to_string());
        }
        // 書き込みテスト（一時ファイル作成を試行）
        let test_path = path.join(".kaku_write_test");
        match std::fs::write(&test_path, "") {
            Ok(_) => {
                let _ = std::fs::remove_file(&test_path);
            }
            Err(_) => {
                return Err("Storage directory is not writable".to_string());
            }
        }
    } else {
        // 親ディレクトリが存在するかチェック
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                return Err(format!(
                    "Parent directory does not exist: {}",
                    parent.display()
                ));
            }
        }
        // 作成を試行
        match std::fs::create_dir_all(path) {
            Ok(_) => {}
            Err(e) => {
                return Err(format!("Cannot create storage directory: {}", e));
            }
        }
    }

    Ok(())
}

/// 設定を更新
#[tauri::command]
pub fn update_settings(state: State<AppState>, settings: SettingsUpdateDto) -> Result<(), String> {
    // storage_directory が指定されている場合は事前に検証
    if let Some(ref storage_directory) = settings.storage_directory {
        validate_storage_directory(storage_directory)?;
    }

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
            if let Some(shortcut) = settings.shortcut_command_palette {
                s.shortcuts.command_palette = shortcut;
            }
            if let Some(shortcut) = settings.shortcut_history_back {
                s.shortcuts.history_back = shortcut;
            }
            if let Some(shortcut) = settings.shortcut_history_forward {
                s.shortcuts.history_forward = shortcut;
            }
            if let Some(shortcut) = settings.shortcut_save_note {
                s.shortcuts.save_note = shortcut;
            }
            if let Some(shortcut) = settings.shortcut_find_in_note {
                s.shortcuts.find_in_note = shortcut;
            }
            if let Some(shortcut) = settings.shortcut_backlink_panel {
                s.shortcuts.backlink_panel = shortcut;
            }
        })
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_validate_storage_directory_relative_path() {
        let result = validate_storage_directory(Path::new("relative/path"));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("absolute path"));
    }

    #[test]
    fn test_validate_storage_directory_path_traversal() {
        let result = validate_storage_directory(Path::new("/home/user/../etc/passwd"));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains(".."));
    }

    #[test]
    fn test_validate_storage_directory_valid() {
        let temp_dir = TempDir::new().unwrap();
        let result = validate_storage_directory(temp_dir.path());
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_storage_directory_creates_new() {
        let temp_dir = TempDir::new().unwrap();
        let new_dir = temp_dir.path().join("new_storage");
        let result = validate_storage_directory(&new_dir);
        assert!(result.is_ok());
        assert!(new_dir.exists());
    }

    #[test]
    fn test_validate_storage_directory_file_not_dir() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("not_a_dir");
        std::fs::write(&file_path, "content").unwrap();

        let result = validate_storage_directory(&file_path);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not a directory"));
    }
}
