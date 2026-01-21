// ホットキー関連コマンド
use crate::AppState;
use tauri::State;

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
