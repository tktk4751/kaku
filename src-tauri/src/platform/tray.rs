use super::hotkey::{mark_window_hidden, mark_window_visible, is_window_visible};
use tauri::{
    image::Image,
    menu::{Menu, MenuItem},
    tray::{TrayIcon, TrayIconBuilder},
    AppHandle, Emitter, Manager, Runtime,
};

// トレイアイコン画像をコンパイル時に埋め込み
static TRAY_ICON_PNG: &[u8] = include_bytes!("../../../kaku.png");

/// PNGデータをRGBAに変換してTauri Image作成
fn load_tray_icon() -> Image<'static> {
    use image::GenericImageView;
    let img = image::load_from_memory(TRAY_ICON_PNG)
        .expect("Failed to decode tray icon PNG");
    let (width, height) = img.dimensions();
    let rgba = img.to_rgba8().into_raw();
    Image::new_owned(rgba, width, height)
}

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
        .icon(load_tray_icon())
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
