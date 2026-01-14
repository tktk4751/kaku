use super::hotkey::{mark_window_hidden, mark_window_visible, is_window_visible};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{TrayIcon, TrayIconBuilder},
    AppHandle, Emitter, Manager, Runtime,
};

/// システムトレイをセットアップ
pub fn setup_tray<R: Runtime>(app: &AppHandle<R>) -> Result<TrayIcon<R>, tauri::Error> {
    // メニューアイテム作成
    let show_item = MenuItem::with_id(app, "show", "表示", true, None::<&str>)?;
    let hide_item = MenuItem::with_id(app, "hide", "非表示", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "終了", true, None::<&str>)?;

    // メニュー作成
    let menu = Menu::with_items(app, &[&show_item, &hide_item, &quit_item])?;

    // トレイアイコン作成
    let tray = TrayIconBuilder::new()
        .icon(app.default_window_icon().cloned().unwrap())
        .tooltip("kaku - クリックで表示/非表示")
        .menu(&menu)
        .on_menu_event(|app, event| {
            match event.id.as_ref() {
                "show" => {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                        mark_window_visible();
                        let _ = window.emit("create-new-note", ());
                        println!("[Tray Menu] Window shown");
                    }
                }
                "hide" => {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.hide();
                        mark_window_hidden();
                        println!("[Tray Menu] Window hidden");
                    }
                }
                "quit" => {
                    println!("[Tray Menu] Quitting...");
                    app.exit(0);
                }
                _ => {}
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let tauri::tray::TrayIconEvent::Click {
                button: tauri::tray::MouseButton::Left,
                button_state: tauri::tray::MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(window) = app.get_webview_window("main") {
                    if is_window_visible() {
                        let _ = window.hide();
                        mark_window_hidden();
                        println!("[Tray Click] Window hidden");
                    } else {
                        let _ = window.show();
                        let _ = window.set_focus();
                        mark_window_visible();
                        let _ = window.emit("create-new-note", ());
                        println!("[Tray Click] Window shown");
                    }
                }
            }
        })
        .build(app)?;

    Ok(tray)
}
