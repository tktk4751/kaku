use crate::domain::WindowGeometry;
use tauri::{Runtime, WebviewWindow};

/// ウィンドウ状態
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowState {
    Visible,
    Hidden,
}

/// ウィンドウマネージャー
pub struct WindowManager {
    state: WindowState,
}

impl WindowManager {
    pub fn new() -> Self {
        Self {
            state: WindowState::Hidden,
        }
    }

    /// ウィンドウの表示/非表示をトグル
    pub fn toggle<R: Runtime>(&mut self, window: &WebviewWindow<R>) -> Result<(), tauri::Error> {
        match self.state {
            WindowState::Visible => {
                window.hide()?;
                self.state = WindowState::Hidden;
            }
            WindowState::Hidden => {
                window.show()?;
                window.set_focus()?;
                self.state = WindowState::Visible;
            }
        }
        Ok(())
    }

    /// ウィンドウを表示
    pub fn show<R: Runtime>(&mut self, window: &WebviewWindow<R>) -> Result<(), tauri::Error> {
        window.show()?;
        window.set_focus()?;
        self.state = WindowState::Visible;
        Ok(())
    }

    /// ウィンドウを非表示
    pub fn hide<R: Runtime>(&mut self, window: &WebviewWindow<R>) -> Result<(), tauri::Error> {
        window.hide()?;
        self.state = WindowState::Hidden;
        Ok(())
    }

    /// 現在の状態を取得
    pub fn state(&self) -> WindowState {
        self.state
    }

    /// ウィンドウからジオメトリを取得（ポップアップウィンドウ用）
    pub fn get_geometry<R: Runtime>(
        window: &WebviewWindow<R>,
    ) -> Result<WindowGeometry, tauri::Error> {
        let position = window.outer_position()?;
        let size = window.outer_size()?;

        Ok(WindowGeometry {
            x: position.x,
            y: position.y,
            width: size.width,
            height: size.height,
            is_maximized: false, // ポップアップは最大化しない
        })
    }

    /// ポップアップウィンドウの最小/最大サイズ制限
    const MIN_WIDTH: u32 = 200;
    const MIN_HEIGHT: u32 = 200;
    const MAX_WIDTH: u32 = 1200;
    const MAX_HEIGHT: u32 = 900;
    const DEFAULT_WIDTH: u32 = 400;
    const DEFAULT_HEIGHT: u32 = 500;

    /// ジオメトリをウィンドウに適用（ポップアップウィンドウ用）
    /// x, y が -1 の場合は中央配置（ただしLinux/Waylandでは位置設定をスキップ）
    /// サイズが制限を超える場合はデフォルト値を使用
    pub fn apply_geometry<R: Runtime>(
        window: &WebviewWindow<R>,
        geometry: &WindowGeometry,
    ) -> Result<(), tauri::Error> {
        use tauri::{LogicalPosition, LogicalSize};

        // サイズをバリデート（最大化状態からの復元時に巨大サイズになるのを防ぐ）
        let (width, height) = if geometry.width > Self::MAX_WIDTH
            || geometry.height > Self::MAX_HEIGHT
            || geometry.width < Self::MIN_WIDTH
            || geometry.height < Self::MIN_HEIGHT
        {
            println!(
                "[WindowManager] Invalid geometry {}x{}, using defaults",
                geometry.width, geometry.height
            );
            (Self::DEFAULT_WIDTH, Self::DEFAULT_HEIGHT)
        } else {
            (geometry.width, geometry.height)
        };

        // サイズを設定
        window.set_size(LogicalSize::new(width, height))?;

        // 位置を設定
        // Linux/Waylandでは初回起動時（x==-1）はウィンドウマネージャーに任せる
        #[cfg(target_os = "linux")]
        {
            if geometry.x != -1 && geometry.y != -1 {
                // 保存された位置がある場合のみ設定を試みる
                let _ = window.set_position(LogicalPosition::new(geometry.x, geometry.y));
            }
            // -1の場合はHyprlandのwindowruleに任せる
        }

        #[cfg(not(target_os = "linux"))]
        {
            // Windows/macOSでは中央配置またはポジション設定
            if geometry.x == -1
                || geometry.y == -1
                || width != geometry.width
                || height != geometry.height
            {
                window.center()?;
            } else {
                window.set_position(LogicalPosition::new(geometry.x, geometry.y))?;
            }
        }

        Ok(())
    }

    /// ウィンドウ位置がモニター範囲内か確認し、必要なら補正
    pub fn ensure_on_screen<R: Runtime>(window: &WebviewWindow<R>) -> Result<(), tauri::Error> {
        let position = window.outer_position()?;
        let size = window.outer_size()?;

        // 現在のモニターを取得
        if let Some(monitor) = window.current_monitor()? {
            let monitor_pos = monitor.position();
            let monitor_size = monitor.size();

            let mut new_x = position.x;
            let mut new_y = position.y;
            let mut needs_move = false;

            // X座標チェック
            if position.x < monitor_pos.x {
                new_x = monitor_pos.x;
                needs_move = true;
            } else if position.x + size.width as i32 > monitor_pos.x + monitor_size.width as i32 {
                new_x = monitor_pos.x + monitor_size.width as i32 - size.width as i32;
                needs_move = true;
            }

            // Y座標チェック
            if position.y < monitor_pos.y {
                new_y = monitor_pos.y;
                needs_move = true;
            } else if position.y + size.height as i32 > monitor_pos.y + monitor_size.height as i32 {
                new_y = monitor_pos.y + monitor_size.height as i32 - size.height as i32;
                needs_move = true;
            }

            if needs_move {
                window.set_position(tauri::LogicalPosition::new(new_x, new_y))?;
            }
        }

        Ok(())
    }
}

impl Default for WindowManager {
    fn default() -> Self {
        Self::new()
    }
}
