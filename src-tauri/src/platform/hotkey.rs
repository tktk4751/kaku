use parking_lot::Mutex;
use std::sync::Arc;
use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

/// ウィンドウ表示状態（Wayland互換のため独自追跡）
/// 初期値はfalse（アプリはトレイ常駐で起動し、ウィンドウは非表示）
static WINDOW_VISIBLE: once_cell::sync::Lazy<Arc<Mutex<bool>>> =
    once_cell::sync::Lazy::new(|| Arc::new(Mutex::new(false)));

/// グローバルホットキーをセットアップ
pub fn setup_global_hotkey<R: Runtime>(app: &AppHandle<R>) -> Result<(), Box<dyn std::error::Error>> {
    // Shift+Space ショートカットを定義
    let shortcut = Shortcut::new(Some(Modifiers::SHIFT), Code::Space);

    println!("[Hotkey] Registering Shift+Space...");

    // ショートカットを登録
    app.global_shortcut().on_shortcut(shortcut, |app, _shortcut, event| {
        if event.state == ShortcutState::Pressed {
            println!("[Hotkey] Shift+Space pressed");

            if let Some(window) = app.get_webview_window("main") {
                let state: tauri::State<crate::AppState> = app.state();
                match crate::services::WindowService::toggle(&window, &state.settings_service) {
                    Ok(result) => println!("[Hotkey] Window toggle: {:?}", result),
                    Err(e) => eprintln!("[Hotkey] Window toggle error: {}", e),
                }
            }
        }
    })?;

    println!("[Hotkey] Registration successful");
    Ok(())
}

/// ウィンドウを非表示にしたことを通知（CloseRequestedイベント用）
pub fn mark_window_hidden() {
    let mut visible = WINDOW_VISIBLE.lock();
    *visible = false;
}

/// ウィンドウを表示したことを通知
pub fn mark_window_visible() {
    let mut visible = WINDOW_VISIBLE.lock();
    *visible = true;
}

/// ウィンドウの表示状態を取得
pub fn is_window_visible() -> bool {
    *WINDOW_VISIBLE.lock()
}
