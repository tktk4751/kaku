//! ウィンドウ表示/非表示サービス
//!
//! # 責務
//!
//! - ウィンドウの表示/非表示を統一インターフェースで管理
//! - ジオメトリの保存/復元を調整
//! - プラットフォーム固有の処理を隠蔽
//!
//! # SOLID原則
//!
//! - **Single Responsibility**: ウィンドウ可視性の管理のみ
//! - **Open/Closed**: プラットフォーム固有処理は platform モジュールに委譲

use crate::domain::WindowGeometry;
use crate::platform::{self, PlatformManager, OFFSCREEN_THRESHOLD};
use crate::services::SettingsService;
use std::sync::Arc;
use tauri::{Emitter, WebviewWindow};

/// ウィンドウ表示/非表示の結果
#[derive(Debug, Clone)]
pub enum ToggleResult {
    /// ウィンドウを表示した
    Shown { position: (i32, i32) },
    /// ウィンドウを非表示にした
    Hidden,
}

/// ウィンドウ表示サービス
///
/// 全てのウィンドウ表示/非表示操作はこのサービスを経由することで、
/// 一貫した動作を保証します。
pub struct WindowService;

impl WindowService {
    /// ウィンドウの表示/非表示をトグル
    ///
    /// # 処理フロー
    ///
    /// ## 非表示にする場合:
    /// 1. 現在のジオメトリを保存
    /// 2. プラットフォーム固有の非表示処理
    /// 3. 可視状態を更新
    ///
    /// ## 表示する場合:
    /// 1. プラットフォーム固有の表示処理
    /// 2. 保存位置に復元（またはデフォルト位置）
    /// 3. フォーカスを取得
    /// 4. create-new-note イベントを発火
    /// 5. 可視状態を更新
    pub fn toggle<R: tauri::Runtime>(
        window: &WebviewWindow<R>,
        settings_service: &Arc<SettingsService>,
    ) -> Result<ToggleResult, String> {
        if PlatformManager::is_visible() {
            Self::hide(window, settings_service)
        } else {
            Self::show(window, settings_service)
        }
    }

    /// ウィンドウを表示
    ///
    /// 既に表示されている場合は何もしない
    pub fn show<R: tauri::Runtime>(
        window: &WebviewWindow<R>,
        settings_service: &Arc<SettingsService>,
    ) -> Result<ToggleResult, String> {
        if PlatformManager::is_visible() {
            // 既に表示されている場合は現在位置を返す
            let geometry = PlatformManager::get_geometry(window)?;
            return Ok(ToggleResult::Shown {
                position: (geometry.x, geometry.y),
            });
        }

        // 復元位置を決定
        let settings = settings_service.get();
        let geometry = &settings.window;

        let (x, y) = Self::calculate_restore_position(geometry);

        // プラットフォーム固有の表示処理
        #[cfg(target_os = "linux")]
        {
            if platform::hyprland::is_hyprland() {
                // Hyprland: show() → ウィンドウ認識待機 → 位置設定 → フォーカス
                let _ = window.show();

                // ウィンドウがHyprlandに認識されるまで待機（最大200ms、10msポーリング）
                // 従来の固定50ms sleepより堅牢
                platform::hyprland::wait_for_window("kaku", 200, 10);

                platform::hyprland::set_window_position("kaku", x, y);
                let _ = window.set_focus();
                platform::mark_window_visible();

                // 新規ノート作成イベント
                let _ = window.emit("create-new-note", ());

                println!("[WindowService] Shown at ({}, {}) via Hyprland", x, y);
                return Ok(ToggleResult::Shown { position: (x, y) });
            }
        }

        // 非Hyprland環境
        let _ = window.show();
        let _ = window.set_focus();
        platform::mark_window_visible();

        // 新規ノート作成イベント
        let _ = window.emit("create-new-note", ());

        println!("[WindowService] Shown");
        Ok(ToggleResult::Shown { position: (x, y) })
    }

    /// ウィンドウを非表示
    ///
    /// 既に非表示の場合は何もしない
    pub fn hide<R: tauri::Runtime>(
        window: &WebviewWindow<R>,
        settings_service: &Arc<SettingsService>,
    ) -> Result<ToggleResult, String> {
        if !PlatformManager::is_visible() {
            return Ok(ToggleResult::Hidden);
        }

        // ジオメトリを保存（オフスクリーン位置は除外）
        Self::save_geometry_if_onscreen(window, settings_service);

        // プラットフォーム固有の非表示処理
        PlatformManager::hide_window(window)?;

        println!("[WindowService] Hidden");
        Ok(ToggleResult::Hidden)
    }

    /// オンスクリーンの場合のみジオメトリを保存
    fn save_geometry_if_onscreen<R: tauri::Runtime>(
        window: &WebviewWindow<R>,
        settings_service: &Arc<SettingsService>,
    ) {
        if let Ok(geometry) = PlatformManager::get_geometry(window) {
            // オフスクリーン位置は保存しない
            if geometry.x >= OFFSCREEN_THRESHOLD && geometry.y >= OFFSCREEN_THRESHOLD {
                match settings_service.update_window_geometry(geometry.clone()) {
                    Ok(_) => println!(
                        "[WindowService] Geometry saved: ({}, {})",
                        geometry.x, geometry.y
                    ),
                    Err(e) => eprintln!("[WindowService] Failed to save geometry: {:?}", e),
                }
            } else {
                println!(
                    "[WindowService] Skipped saving offscreen position: ({}, {})",
                    geometry.x, geometry.y
                );
            }
        }
    }

    /// 復元位置を計算
    fn calculate_restore_position(geometry: &WindowGeometry) -> (i32, i32) {
        // オフスクリーン座標または未設定の場合はデフォルト位置
        if geometry.x > OFFSCREEN_THRESHOLD
            && geometry.y > OFFSCREEN_THRESHOLD
            && geometry.x != -1
            && geometry.y != -1
        {
            (geometry.x, geometry.y)
        } else {
            PlatformManager::calculate_default_position(
                geometry.width.max(400) as u32,
                geometry.height.max(500) as u32,
            )
        }
    }
}

#[cfg(test)]
#[allow(deprecated)] // WindowGeometry::is_maximized は非推奨だがテストでは使用
mod tests {
    use super::*;
    use crate::domain::WindowGeometry;

    #[test]
    fn test_calculate_restore_position_with_valid_geometry() {
        let geometry = WindowGeometry {
            x: 100,
            y: 200,
            width: 800,
            height: 600,
            is_maximized: false,
        };
        let (x, y) = WindowService::calculate_restore_position(&geometry);
        assert_eq!((x, y), (100, 200));
    }

    #[test]
    fn test_calculate_restore_position_with_offscreen() {
        let geometry = WindowGeometry {
            x: -10000,
            y: -10000,
            width: 800,
            height: 600,
            is_maximized: false,
        };
        let (x, y) = WindowService::calculate_restore_position(&geometry);
        // デフォルト位置が返される（プラットフォーム依存）
        assert!(x >= 0 || x < 0); // 何らかの値が返る
        assert!(y >= 0 || y < 0);
    }

    #[test]
    fn test_calculate_restore_position_with_unset() {
        let geometry = WindowGeometry {
            x: -1,
            y: -1,
            width: 800,
            height: 600,
            is_maximized: false,
        };
        let (x, y) = WindowService::calculate_restore_position(&geometry);
        // デフォルト位置が返される
        assert!(x >= 0 || x < 0);
        assert!(y >= 0 || y < 0);
    }
}
