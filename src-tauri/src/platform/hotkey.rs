use parking_lot::Mutex;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, Runtime};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

/// ウィンドウ表示状態（Wayland互換のため独自追跡）
static WINDOW_VISIBLE: once_cell::sync::Lazy<Arc<Mutex<bool>>> =
    once_cell::sync::Lazy::new(|| Arc::new(Mutex::new(true)));

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
                let mut visible = WINDOW_VISIBLE.lock();

                #[cfg(target_os = "linux")]
                {
                    if super::hyprland::is_hyprland() {
                        if *visible {
                            // 非表示: オフスクリーンに移動
                            super::hyprland::move_offscreen("kaku");
                            *visible = false;
                            println!("[Hotkey] Window moved offscreen");
                        } else {
                            // 表示: 保存位置に移動
                            let state: tauri::State<crate::AppState> = app.state();
                            let settings = state.settings_service.get();
                            let geometry = &settings.window;

                            // オフスクリーン座標（非表示位置）または未設定の場合はデフォルト位置を使用
                            let (x, y) = if geometry.x > -5000 && geometry.y > -5000 && geometry.x != -1 && geometry.y != -1 {
                                (geometry.x, geometry.y)
                            } else {
                                super::hyprland::calculate_default_position(400, 500)
                                    .unwrap_or((100, 50))
                            };

                            super::hyprland::set_window_position("kaku", x, y);
                            let _ = window.set_focus();
                            let _ = window.emit("create-new-note", ());
                            *visible = true;
                            println!("[Hotkey] Window moved to ({}, {})", x, y);
                        }
                        return;
                    }
                }

                // 非Hyprland環境
                if *visible {
                    let _ = window.hide();
                    *visible = false;
                    println!("[Hotkey] Window hidden");
                } else {
                    let _ = window.show();
                    let _ = window.set_focus();
                    *visible = true;
                    println!("[Hotkey] Window shown");
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
