// kaku - Fast Markdown Memo Application
// Architecture: SOLID principles with layered design

pub mod domain;
pub mod traits;
pub mod infrastructure;
pub mod services;
pub mod platform;
pub mod commands;

use infrastructure::{EventBusImpl, FileNoteRepository, FileStorage, HeadingFilenameStrategy};
use parking_lot::Mutex;
use platform::{setup_global_hotkey, setup_tray};
use services::{NoteService, SettingsService};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};

/// グローバルなAppHandle参照（IPC用）
static APP_HANDLE: once_cell::sync::OnceCell<Mutex<Option<AppHandle<tauri::Wry>>>> =
    once_cell::sync::OnceCell::new();

/// ウィンドウジオメトリを保存（プラットフォーム対応）
/// オフスクリーン位置（非表示状態）は保存しない
fn save_window_geometry_impl<R: tauri::Runtime>(
    window: &tauri::WebviewWindow<R>,
    settings_service: &Arc<SettingsService>,
) {
    #[cfg(target_os = "linux")]
    {
        // Linux: Hyprland（Wayland）ではhyprctlを使用、それ以外はTauri API
        if platform::hyprland::is_hyprland() {
            if let Some((x, y)) = platform::hyprland::get_window_position("kaku") {
                // オフスクリーン位置は保存しない
                if x < -5000 || y < -5000 {
                    println!("[Geometry] Skipped saving offscreen position: ({}, {})", x, y);
                    return;
                }
                let mut geometry = platform::WindowManager::get_geometry(window)
                    .unwrap_or_default();
                geometry.x = x;
                geometry.y = y;
                match settings_service.update_window_geometry(geometry) {
                    Ok(_) => println!("[Geometry] Saved via hyprctl: ({}, {})", x, y),
                    Err(e) => eprintln!("[Geometry] ERROR saving: {:?}", e),
                }
                return;
            }
        }
        // X11またはhyprctl失敗時はTauri APIを使用
        if let Ok(geometry) = platform::WindowManager::get_geometry(window) {
            let _ = settings_service.update_window_geometry(geometry);
            println!("[Geometry] Saved via Tauri API");
        }
    }

    #[cfg(not(target_os = "linux"))]
    {
        // Windows/macOS: Tauri APIを使用
        if let Ok(geometry) = platform::WindowManager::get_geometry(window) {
            let _ = settings_service.update_window_geometry(geometry);
            println!("[Geometry] Saved via Tauri API");
        }
    }
}

/// ウィンドウ位置を復元（プラットフォーム対応）
fn restore_window_position_impl(settings_service: &Arc<SettingsService>) {
    let settings = settings_service.get();
    let geometry = &settings.window;

    // 位置が保存されていない場合はスキップ
    if geometry.x == -1 && geometry.y == -1 {
        return;
    }

    #[cfg(target_os = "linux")]
    {
        // Linux: Hyprland（Wayland）ではhyprctlを使用
        if platform::hyprland::is_hyprland() {
            let x = geometry.x;
            let y = geometry.y;
            // ウィンドウが表示された後に位置を設定（set_window_position内でピン処理も行う）
            std::thread::spawn(move || {
                // 最小限の遅延（ウィンドウがマップされるのを待つ）
                std::thread::sleep(std::time::Duration::from_millis(50));
                platform::hyprland::set_window_position("kaku", x, y);
                println!("[Geometry] Restored via hyprctl: ({}, {})", x, y);
            });
        }
        // X11ではTauriが自動的に位置を適用するので追加処理不要
    }

    #[cfg(not(target_os = "linux"))]
    {
        // Windows/macOS: Tauriが起動時にapply_geometryで位置を設定済み
        // 追加処理不要
        let _ = geometry; // unused warning回避
    }
}

/// IPCからウィンドウをトグル
fn toggle_window_from_ipc() {
    if let Some(handle_mutex) = APP_HANDLE.get() {
        if let Some(ref handle) = *handle_mutex.lock() {
            if let Some(window) = handle.get_webview_window("main") {
                if platform::is_window_visible() {
                    // ジオメトリを保存
                    let state: tauri::State<AppState> = handle.state();
                    save_window_geometry_impl(&window, &state.settings_service);

                    #[cfg(target_os = "linux")]
                    {
                        // Hyprlandの場合、オフスクリーンに移動して非表示
                        if platform::hyprland::is_hyprland() {
                            platform::hyprland::move_offscreen("kaku");
                            platform::mark_window_hidden();
                            println!("[IPC] Window moved offscreen (hidden)");
                            return;
                        }
                    }

                    let _ = window.hide();
                    platform::mark_window_hidden();
                    println!("[IPC] Window hidden");
                } else {
                    #[cfg(target_os = "linux")]
                    {
                        // Hyprlandの場合
                        if platform::hyprland::is_hyprland() {
                            let state: tauri::State<AppState> = handle.state();
                            let settings = state.settings_service.get();
                            let geometry = &settings.window;

                            // オフスクリーン座標（非表示位置）または未設定の場合はデフォルト位置を使用
                            let (x, y) = if geometry.x > -5000 && geometry.y > -5000 && geometry.x != -1 && geometry.y != -1 {
                                (geometry.x, geometry.y)
                            } else {
                                platform::hyprland::calculate_default_position(400, 500)
                                    .unwrap_or((100, 50))
                            };

                            // オフスクリーンからの復帰時はshow()を先に呼ぶ
                            // （Hyprlandがウィンドウを認識するため）
                            let _ = window.show();

                            // ウィンドウがHyprlandに認識されるまで少し待つ
                            std::thread::sleep(std::time::Duration::from_millis(50));

                            platform::hyprland::set_window_position("kaku", x, y);
                            let _ = window.set_focus();
                            platform::mark_window_visible();

                            // フロントエンドに新規ノート作成イベントを送信
                            let _ = window.emit("create-new-note", ());
                            println!("[IPC] Window moved to ({}, {})", x, y);
                            return;
                        }
                    }

                    let _ = window.show();
                    let _ = window.set_focus();
                    platform::mark_window_visible();

                    // 保存された位置に復元
                    let state: tauri::State<AppState> = handle.state();
                    restore_window_position_impl(&state.settings_service);

                    // フロントエンドに新規ノート作成イベントを送信
                    let _ = window.emit("create-new-note", ());
                    println!("[IPC] Window shown, emitted create-new-note");
                }
            }
        }
    }
}

/// アプリケーション状態（Dependency Injection Container）
pub struct AppState {
    pub note_service: NoteService,
    pub settings_service: Arc<SettingsService>,
    pub event_bus: Arc<EventBusImpl>,
}

impl AppState {
    pub fn new() -> Self {
        // EventBus
        let event_bus = Arc::new(EventBusImpl::new());

        // Settings Service
        let settings_service = Arc::new(SettingsService::new(event_bus.clone()));

        // Storage & Repository
        let storage = Arc::new(FileStorage::new());
        let filename_strategy = Arc::new(HeadingFilenameStrategy::new());
        let repository = Arc::new(FileNoteRepository::new(
            storage,
            filename_strategy,
            settings_service.clone(),
        ));

        // Note Service
        let note_service = NoteService::new(repository, event_bus.clone());

        Self {
            note_service,
            settings_service,
            event_bus,
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

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
                            std::thread::sleep(std::time::Duration::from_millis(50));
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

                // 閉じるボタンで非表示にする（終了しない）+ ジオメトリ保存
                let window_clone = window.clone();
                let settings_service_clone = settings_service.settings_service.clone();
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();

                        // ジオメトリを保存
                        #[cfg(target_os = "linux")]
                        {
                            // Linux/Waylandではhyprctlから実際の位置を取得
                            if let Some((x, y)) = platform::hyprland::get_window_position("kaku") {
                                let mut geometry = platform::WindowManager::get_geometry(&window_clone)
                                    .unwrap_or_default();
                                geometry.x = x;
                                geometry.y = y;
                                let _ = settings_service_clone.update_window_geometry(geometry);
                                println!("[CloseButton] Geometry saved via hyprctl: ({}, {})", x, y);
                            }
                        }

                        #[cfg(not(target_os = "linux"))]
                        {
                            if let Ok(geometry) = platform::WindowManager::get_geometry(&window_clone) {
                                let _ = settings_service_clone.update_window_geometry(geometry);
                                println!("[CloseButton] Geometry saved");
                            }
                        }

                        let _ = window_clone.hide();
                        platform::mark_window_hidden();
                        println!("[CloseButton] Window hidden");
                    }
                });
            } else {
                eprintln!("[Startup] ERROR: Could not get main window!");
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::create_note,
            commands::save_note,
            commands::load_note,
            commands::delete_note,
            commands::list_notes,
            commands::get_settings,
            commands::update_settings,
            commands::save_window_geometry,
            commands::prepare_hide,
            commands::set_last_note_uid,
            commands::quit_app,
            commands::hide_window,
            commands::toggle_maximize,
            commands::update_hotkey,
            commands::get_current_hotkey,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
