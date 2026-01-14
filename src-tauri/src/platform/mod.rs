pub mod tray;
pub mod hotkey;
pub mod window;
pub mod ipc;
#[cfg(target_os = "linux")]
pub mod hyprland;

pub use tray::setup_tray;
pub use hotkey::{setup_global_hotkey, mark_window_hidden, mark_window_visible, is_window_visible};
pub use window::WindowManager;
pub use ipc::{send_command, is_instance_running, start_ipc_server, cleanup as cleanup_ipc};
