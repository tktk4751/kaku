pub mod tray;
pub mod hotkey;
pub mod window;
pub mod ipc;
pub mod manager;
#[cfg(target_os = "linux")]
pub mod hyprland;

pub use tray::setup_tray;
pub use hotkey::{setup_global_hotkey, mark_window_hidden, mark_window_visible, is_window_visible};
pub use window::WindowManager;
pub use ipc::{send_command, is_instance_running, start_ipc_server, cleanup as cleanup_ipc};
pub use manager::PlatformManager;

// ===== オフスクリーン座標定数 =====
// Hyprlandでウィンドウを非表示にする際、画面外に移動する座標
// 判定閾値（-5000）と移動先座標（-10000）を明確に分離
/// オフスクリーン位置の判定閾値（この値未満ならオフスクリーンと判定）
pub const OFFSCREEN_THRESHOLD: i32 = -5000;
/// オフスクリーンに移動する際の座標
pub const OFFSCREEN_POSITION: i32 = -10000;
