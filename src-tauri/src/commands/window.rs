// ウィンドウ関連コマンド
use crate::AppState;
use tauri::{AppHandle, Manager, State};

/// ウィンドウジオメトリを保存
#[tauri::command]
pub fn save_window_geometry(
    app: AppHandle,
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
    app: AppHandle,
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
pub fn quit_app(app: AppHandle) {
    app.exit(0);
}

/// ウィンドウを非表示にする
#[tauri::command]
pub fn hide_window(app: AppHandle, state: State<AppState>) -> Result<(), String> {
    println!("[hide_window] Command called");
    if let Some(window) = app.get_webview_window("main") {
        let is_visible = crate::platform::is_window_visible();
        println!("[hide_window] is_visible = {}", is_visible);
        match crate::services::WindowService::hide(&window, &state.settings_service) {
            Ok(result) => {
                println!("[hide_window] Success: {:?}", result);
                Ok(())
            }
            Err(e) => {
                eprintln!("[hide_window] Error: {}", e);
                Err(e)
            }
        }
    } else {
        eprintln!("[hide_window] Window not found");
        Err("Window not found".to_string())
    }
}

/// ウィンドウを最大化/元に戻す
#[tauri::command]
pub fn toggle_maximize(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        if window.is_maximized().unwrap_or(false) {
            window.unmaximize().map_err(|e| e.to_string())?;
        } else {
            window.maximize().map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}
