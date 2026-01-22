// PlatformManager Facade
//
// SOLID: Facade Pattern
// プラットフォーム固有のウィンドウ操作を統一インターフェースで提供
//
// 注意: このFacadeは純粋なウィンドウ操作のみを担当。
// イベント発火（create-new-note等）やジオメトリ保存は呼び出し元が行う。

use super::{hyprland, mark_window_hidden, mark_window_visible, is_window_visible, WindowManager};
use crate::domain::WindowGeometry;

/// プラットフォーム操作の統一インターフェース
pub struct PlatformManager;

impl PlatformManager {
    /// ウィンドウを表示（可視状態を追跡）
    pub fn show_window<R: tauri::Runtime>(window: &tauri::WebviewWindow<R>) -> Result<(), String> {
        #[cfg(target_os = "linux")]
        {
            if hyprland::is_hyprland() {
                let _ = window.show();
                std::thread::sleep(std::time::Duration::from_millis(50));
                mark_window_visible();
                return Ok(());
            }
        }
        window.show().map_err(|e| e.to_string())?;
        mark_window_visible();
        Ok(())
    }

    /// ウィンドウを非表示（可視状態を追跡）
    /// 注意: ジオメトリ保存は呼び出し元が事前に行うこと
    pub fn hide_window<R: tauri::Runtime>(window: &tauri::WebviewWindow<R>) -> Result<(), String> {
        #[cfg(target_os = "linux")]
        {
            if hyprland::is_hyprland() {
                hyprland::move_offscreen("kaku");
                mark_window_hidden();
                return Ok(());
            }
        }
        window.hide().map_err(|e| e.to_string())?;
        mark_window_hidden();
        Ok(())
    }

    /// ウィンドウジオメトリを取得（プラットフォーム対応）
    pub fn get_geometry<R: tauri::Runtime>(window: &tauri::WebviewWindow<R>) -> Result<WindowGeometry, String> {
        #[cfg(target_os = "linux")]
        {
            if hyprland::is_hyprland() {
                let mut geometry = WindowManager::get_geometry(window)
                    .map_err(|e| e.to_string())?;
                if let Some((x, y)) = hyprland::get_window_position("kaku") {
                    // オフスクリーン位置は無視
                    if x >= super::OFFSCREEN_THRESHOLD && y >= super::OFFSCREEN_THRESHOLD {
                        geometry.x = x;
                        geometry.y = y;
                    }
                }
                return Ok(geometry);
            }
        }
        WindowManager::get_geometry(window).map_err(|e| e.to_string())
    }

    /// ウィンドウ位置を設定
    pub fn set_position(x: i32, y: i32) {
        #[cfg(target_os = "linux")]
        {
            if hyprland::is_hyprland() {
                hyprland::set_window_position("kaku", x, y);
                return;
            }
        }
        // X11/Windows/macOS: Tauri handles this via window methods
        let _ = (x, y); // suppress unused warning on non-Linux
    }

    /// 現在ウィンドウが可視かどうか
    pub fn is_visible() -> bool {
        is_window_visible()
    }

    /// デフォルトウィンドウ位置を計算
    pub fn calculate_default_position(width: u32, height: u32) -> (i32, i32) {
        #[cfg(target_os = "linux")]
        {
            if hyprland::is_hyprland() {
                return hyprland::calculate_default_position(width as i32, height as i32)
                    .unwrap_or((100, 50));
            }
        }
        let _ = (width, height); // suppress unused warning
        (100, 50)
    }

    /// ウィンドウサイズを設定（Hyprland 専用）
    #[cfg(target_os = "linux")]
    pub fn set_window_size(width: u32, height: u32) {
        if hyprland::is_hyprland() {
            hyprland::set_window_size("kaku", width, height);
        }
    }

    /// オフスクリーンに移動（Hyprland 専用）
    #[cfg(target_os = "linux")]
    pub fn move_offscreen() {
        if hyprland::is_hyprland() {
            hyprland::move_offscreen("kaku");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 可視状態の追跡が正しく動作することを確認
    #[test]
    fn test_is_visible_tracking() {
        // Note: この値は他のテストの影響を受ける可能性がある
        // アプリ起動時の初期値はfalse（トレイ常駐で起動）
        let initial = PlatformManager::is_visible();

        // 状態を変更してテスト
        mark_window_visible();
        assert!(PlatformManager::is_visible(), "mark_window_visible() should set visible to true");

        mark_window_hidden();
        assert!(!PlatformManager::is_visible(), "mark_window_hidden() should set visible to false");

        // 元の状態に戻す（他のテストへの影響を最小化）
        if initial {
            mark_window_visible();
        }
    }

    /// デフォルト位置の計算が妥当な値を返すことを確認
    #[test]
    fn test_calculate_default_position() {
        let (x, y) = PlatformManager::calculate_default_position(400, 500);

        // 非Hyprland環境では固定値 (100, 50) が返される
        // Hyprland環境ではモニター右端の位置が返される
        if !hyprland::is_hyprland() {
            assert_eq!((x, y), (100, 50), "Non-Hyprland should return (100, 50)");
        } else {
            // Hyprlandの場合、xは負の大きな値にならないはず
            // (モニター幅 - ウィンドウ幅 - マージン >= 0 を想定)
            assert!(x >= -10000, "x position should be reasonable");
            assert!(y >= 0, "y position should be non-negative");
        }
    }

    /// 可視状態の連続的な変更が正しく追跡されることを確認
    #[test]
    fn test_visibility_state_changes() {
        // 状態を既知の値に設定
        mark_window_hidden();
        assert!(!PlatformManager::is_visible());

        // 複数回のトグル
        mark_window_visible();
        assert!(PlatformManager::is_visible());

        mark_window_visible(); // 二重呼び出し
        assert!(PlatformManager::is_visible(), "Double visible call should still be visible");

        mark_window_hidden();
        assert!(!PlatformManager::is_visible());

        mark_window_hidden(); // 二重呼び出し
        assert!(!PlatformManager::is_visible(), "Double hidden call should still be hidden");
    }
}
