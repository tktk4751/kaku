// kaku - Fast Markdown Memo Application
// Architecture: SOLID principles with layered design

pub mod domain;
pub mod traits;
pub mod infrastructure;
pub mod services;
pub mod platform;
pub mod commands;
pub mod app_state;

use parking_lot::Mutex;
use platform::{setup_global_hotkey, setup_tray};
use tauri::{AppHandle, Manager};

// AppState を re-export
pub use app_state::AppState;

/// グローバルなAppHandle参照（IPC用）
static APP_HANDLE: once_cell::sync::OnceCell<Mutex<Option<AppHandle<tauri::Wry>>>> =
    once_cell::sync::OnceCell::new();

/// IPCからウィンドウをトグル
fn toggle_window_from_ipc() {
    if let Some(handle_mutex) = APP_HANDLE.get() {
        if let Some(ref handle) = *handle_mutex.lock() {
            if let Some(window) = handle.get_webview_window("main") {
                let state: tauri::State<AppState> = handle.state();
                match services::WindowService::toggle(&window, &state.settings_service) {
                    Ok(result) => println!("[IPC] Window toggle: {:?}", result),
                    Err(e) => eprintln!("[IPC] Window toggle error: {}", e),
                }
            }
        }
    }
}

// AppState は app_state.rs に移動済み

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // NVIDIA + Wayland + webkitgtk の互換性問題を回避
    // DMABUFレンダラーを無効化（explicit sync問題の回避）
    #[cfg(target_os = "linux")]
    {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // アプリケーション状態を初期化
            let state = AppState::new();
            app.manage(state);

            // システムトレイをセットアップ
            setup_tray(app.handle())?;

            // グローバルホットキーをセットアップ
            match setup_global_hotkey(app.handle()) {
                Ok(_) => println!("[Startup] Global hotkey registered: Shift+Space"),
                Err(e) => eprintln!("[Startup] ERROR: Failed to setup global hotkey: {}", e),
            }

            // AppHandleを保存（IPC用）
            let _ = APP_HANDLE.set(Mutex::new(Some(app.handle().clone())));

            // IPCサーバーを起動
            match platform::start_ipc_server(toggle_window_from_ipc) {
                Ok(_) => println!("[Startup] IPC server started"),
                Err(e) => eprintln!("[Startup] ERROR: Failed to start IPC server: {}", e),
            }

            // ウィンドウ設定を適用
            if let Some(window) = app.get_webview_window("main") {
                let settings_service = app.state::<AppState>();
                let settings = settings_service.settings_service.get();

                // 保存されたジオメトリを復元（初回起動時はデフォルト値）
                let geometry = &settings.window;

                #[cfg(target_os = "linux")]
                {
                    // Linux/Waylandでは、ジオメトリ操作を最小限に
                    if geometry.width > 0 && geometry.height > 0 {
                        let _ = platform::WindowManager::apply_geometry(&window, geometry);
                    }

                    // 起動時は非表示（トレイ常駐）
                    platform::mark_window_hidden();

                    // Hyprlandの場合、オフスクリーンに移動して非表示状態で待機
                    if platform::hyprland::is_hyprland() {
                        let width = geometry.width.max(400);
                        let height = geometry.height.max(500);
                        std::thread::spawn(move || {
                            // ウィンドウがHyprlandに認識されるまで待機（最大200ms）
                            platform::hyprland::wait_for_window("kaku", 200, 10);
                            platform::hyprland::set_window_size("kaku", width, height);
                            // オフスクリーンに移動（非表示状態）
                            platform::hyprland::move_offscreen("kaku");
                            println!("[Startup] Window moved offscreen (tray mode)");
                        });
                    }
                    println!("[Startup] Started in tray mode (window hidden)");
                }

                #[cfg(not(target_os = "linux"))]
                {
                    if geometry.width > 0 && geometry.height > 0 {
                        let _ = platform::WindowManager::apply_geometry(&window, geometry);
                    }
                    // 起動時は非表示（トレイ常駐）
                    let _ = window.hide();
                    platform::mark_window_hidden();
                    println!("[Startup] Started in tray mode (window hidden)");
                }

                // 閉じるボタンで非表示にする（終了しない）
                // WindowService::hide() を使用してジオメトリ保存と非表示を統一
                let window_clone = window.clone();
                let settings_service_clone = settings_service.settings_service.clone();
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();

                        // WindowService を使用して非表示（ジオメトリ保存も含む）
                        match services::WindowService::hide(&window_clone, &settings_service_clone) {
                            Ok(_) => println!("[CloseButton] Window hidden via WindowService"),
                            Err(e) => eprintln!("[CloseButton] Error: {}", e),
                        }
                    }
                });
            } else {
                eprintln!("[Startup] ERROR: Could not get main window!");
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Note commands
            commands::note::create_note,
            commands::note::save_note,
            commands::note::load_note,
            commands::note::delete_note,
            commands::note::list_notes,
            commands::note::search_notes,
            commands::note::resolve_wiki_link,
            // Backlink commands
            commands::backlink::get_backlinks,
            commands::backlink::rebuild_backlink_index,
            // Settings commands
            commands::settings::get_settings,
            commands::settings::update_settings,
            // Window commands
            commands::window::save_window_geometry,
            commands::window::prepare_hide,
            commands::window::set_last_note_uid,
            commands::window::quit_app,
            commands::window::hide_window,
            commands::window::toggle_maximize,
            // Hotkey commands
            commands::hotkey::update_hotkey,
            commands::hotkey::get_current_hotkey,
            // Gallery commands
            commands::gallery::list_notes_gallery,
            // Tag commands
            commands::tag::get_all_tags,
            commands::tag::get_note_tags,
            commands::tag::update_note_tags,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
