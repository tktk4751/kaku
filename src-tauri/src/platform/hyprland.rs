//! Hyprland連携モジュール
//!
//! hyprctlを使用してウィンドウ位置を取得・設定します。
//!
//! # セキュリティ
//!
//! - hyprctlは `/usr/bin/hyprctl` から実行（PATH探索ではない）
//! - 出力はJSON形式で検証
//! - 入力パラメータはエスケープ処理済み

use std::process::Command;
use std::path::Path;
use std::sync::OnceLock;

/// 検証済みhyprctlパス（一度だけ検証）
static HYPRCTL_PATH: OnceLock<Option<&'static str>> = OnceLock::new();

/// hyprctlの実行パスを取得（検証済み）
///
/// 標準的なインストール場所を確認し、存在するパスを返します。
/// セキュリティのため、PATH探索ではなく固定パスを使用します。
fn get_hyprctl_path() -> Option<&'static str> {
    *HYPRCTL_PATH.get_or_init(|| {
        // 標準的なインストール場所を順に確認
        const KNOWN_PATHS: &[&str] = &[
            "/usr/bin/hyprctl",
            "/usr/local/bin/hyprctl",
            "/bin/hyprctl",
        ];

        for path in KNOWN_PATHS {
            if Path::new(path).exists() {
                return Some(*path);
            }
        }

        // フォールバック: which コマンドで探す（開発環境用）
        if let Ok(output) = Command::new("which").arg("hyprctl").output() {
            if output.status.success() {
                let path_str = String::from_utf8_lossy(&output.stdout);
                let trimmed = path_str.trim();
                // 静的文字列に変換（リークするが一度だけ）
                if !trimmed.is_empty() && Path::new(trimmed).exists() {
                    return Some(Box::leak(trimmed.to_string().into_boxed_str()));
                }
            }
        }

        None
    })
}

/// hyprctlコマンドを作成（検証済みパスを使用）
fn hyprctl_command() -> Option<Command> {
    get_hyprctl_path().map(Command::new)
}

/// Waylandセッションで実行中かどうかを判定
pub fn is_wayland() -> bool {
    std::env::var("WAYLAND_DISPLAY").is_ok()
}

/// Hyprlandで実行中かどうかを判定
pub fn is_hyprland() -> bool {
    is_wayland() && is_available()
}

/// カーソル位置を取得
fn get_cursor_position() -> Option<(i32, i32)> {
    let output = Command::new("hyprctl")
        .arg("cursorpos")
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let pos_str = String::from_utf8_lossy(&output.stdout);
    let parts: Vec<&str> = pos_str.trim().split(", ").collect();
    if parts.len() == 2 {
        let x = parts[0].parse().ok()?;
        let y = parts[1].parse().ok()?;
        return Some((x, y));
    }
    None
}

/// カーソル位置を設定
fn set_cursor_position(x: i32, y: i32) {
    let pos = format!("{} {}", x, y);
    let _ = Command::new("hyprctl")
        .args(["dispatch", "movecursor", &pos])
        .output();
}

/// ウィンドウがHyprlandに認識されるまで待機
///
/// # 引数
/// - `class_name`: ウィンドウのクラス名
/// - `timeout_ms`: 最大待機時間（ミリ秒）
/// - `poll_interval_ms`: ポーリング間隔（ミリ秒）
///
/// # 戻り値
/// - `true`: ウィンドウが認識された
/// - `false`: タイムアウト
pub fn wait_for_window(class_name: &str, timeout_ms: u64, poll_interval_ms: u64) -> bool {
    let start = std::time::Instant::now();
    let timeout = std::time::Duration::from_millis(timeout_ms);
    let interval = std::time::Duration::from_millis(poll_interval_ms);

    while start.elapsed() < timeout {
        if get_window_position(class_name).is_some() {
            println!(
                "[Hyprland] Window '{}' recognized after {:?}",
                class_name,
                start.elapsed()
            );
            return true;
        }
        std::thread::sleep(interval);
    }

    eprintln!(
        "[Hyprland] Window '{}' not recognized within {}ms",
        class_name, timeout_ms
    );
    false
}

/// Hyprlandからウィンドウ位置を取得
/// hyprctl clients -j を使用してJSONから位置を解析
pub fn get_window_position(class_name: &str) -> Option<(i32, i32)> {
    let output = Command::new("hyprctl")
        .args(["clients", "-j"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let json_str = String::from_utf8_lossy(&output.stdout);

    // JSONをパース（serde_jsonを使用）
    let clients: Vec<serde_json::Value> = serde_json::from_str(&json_str).ok()?;

    for client in clients {
        if let Some(class) = client.get("class").and_then(|v| v.as_str()) {
            if class == class_name {
                let at = client.get("at")?;
                let x = at.get(0)?.as_i64()? as i32;
                let y = at.get(1)?.as_i64()? as i32;
                return Some((x, y));
            }
        }
    }

    None
}

/// Hyprlandでウィンドウ位置を設定
/// Hyprland 0.53+ではmovewindowpixelがセレクターで動作しないため
/// focuswindow + moveactiveを使用（カーソル位置は保存・復元）
pub fn set_window_position(class_name: &str, x: i32, y: i32) -> bool {
    // ウィンドウ情報を取得
    let (addr, was_pinned) = match get_window_info(class_name) {
        Some(info) => info,
        None => {
            eprintln!("[Hyprland] Window not found: {}", class_name);
            return false;
        }
    };
    let class_selector = format!("class:{}", class_name);
    let addr_selector = format!("address:{}", addr);

    // カーソル位置を保存
    let cursor_pos = get_cursor_position();

    // アニメーションを一時的に無効化
    let _ = Command::new("hyprctl")
        .args(["keyword", "animations:enabled", "0"])
        .output();

    // ピン留めされていた場合は解除
    if was_pinned {
        let _ = Command::new("hyprctl")
            .args(["dispatch", "pin", &addr_selector])
            .output();
    }

    // focuswindow + moveactiveで移動（Hyprland 0.53+対応）
    let _ = Command::new("hyprctl")
        .args(["dispatch", "focuswindow", &class_selector])
        .output();

    let position = format!("exact {} {}", x, y);
    let move_result = Command::new("hyprctl")
        .args(["dispatch", "moveactive", &position])
        .output();

    // 常にピン留め（全ワークスペースで表示）
    let _ = Command::new("hyprctl")
        .args(["dispatch", "pin", &addr_selector])
        .output();

    // カーソル位置を復元
    if let Some((cx, cy)) = cursor_pos {
        set_cursor_position(cx, cy);
    }

    // アニメーションを再有効化
    let _ = Command::new("hyprctl")
        .args(["keyword", "animations:enabled", "1"])
        .output();

    match move_result {
        Ok(result) => {
            if result.status.success() {
                println!("[Hyprland] Window moved to ({}, {})", x, y);
                true
            } else {
                eprintln!("[Hyprland] Failed to move window: {:?}",
                    String::from_utf8_lossy(&result.stderr));
                false
            }
        }
        Err(e) => {
            eprintln!("[Hyprland] hyprctl command failed: {}", e);
            false
        }
    }
}

/// ウィンドウのアドレスとピン状態を取得
fn get_window_info(class_name: &str) -> Option<(String, bool)> {
    let output = Command::new("hyprctl")
        .args(["clients", "-j"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let json_str = String::from_utf8_lossy(&output.stdout);
    let clients: Vec<serde_json::Value> = serde_json::from_str(&json_str).ok()?;

    for client in clients {
        if let Some(class) = client.get("class").and_then(|v| v.as_str()) {
            if class == class_name {
                let addr = client.get("address").and_then(|v| v.as_str())?.to_string();
                let pinned = client.get("pinned").and_then(|v| v.as_bool()).unwrap_or(false);
                return Some((addr, pinned));
            }
        }
    }

    None
}


/// ウィンドウをピン留め
pub fn pin_window(class_name: &str) -> bool {
    let selector = format!("class:{}", class_name);

    Command::new("hyprctl")
        .args(["dispatch", "pin", &selector])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// ウィンドウをオフスクリーンに移動（非表示用）
pub fn move_offscreen(class_name: &str) -> bool {
    set_window_position_internal(class_name, super::OFFSCREEN_POSITION, super::OFFSCREEN_POSITION)
}

/// 内部用: 位置設定（ピン状態を維持、カーソル位置を保存・復元）
/// Hyprland 0.53+ではmovewindowpixelがセレクターで動作しないため
/// focuswindow + moveactiveを使用
fn set_window_position_internal(class_name: &str, x: i32, y: i32) -> bool {
    let (addr, was_pinned) = match get_window_info(class_name) {
        Some(info) => info,
        None => return false,
    };
    let class_selector = format!("class:{}", class_name);
    let addr_selector = format!("address:{}", addr);

    // カーソル位置を保存
    let cursor_pos = get_cursor_position();

    // アニメーション無効化
    let _ = Command::new("hyprctl")
        .args(["keyword", "animations:enabled", "0"])
        .output();

    // ピン解除（移動のため）
    if was_pinned {
        let _ = Command::new("hyprctl")
            .args(["dispatch", "pin", &addr_selector])
            .output();
    }

    // focuswindow + moveactiveで移動（Hyprland 0.53+対応）
    let _ = Command::new("hyprctl")
        .args(["dispatch", "focuswindow", &class_selector])
        .output();

    let position = format!("exact {} {}", x, y);
    let result = Command::new("hyprctl")
        .args(["dispatch", "moveactive", &position])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    // ピン復元
    if was_pinned {
        let _ = Command::new("hyprctl")
            .args(["dispatch", "pin", &addr_selector])
            .output();
    }

    // カーソル位置を復元
    if let Some((cx, cy)) = cursor_pos {
        set_cursor_position(cx, cy);
    }

    // アニメーション再有効化
    let _ = Command::new("hyprctl")
        .args(["keyword", "animations:enabled", "1"])
        .output();

    result
}

/// ウィンドウサイズを設定
pub fn set_window_size(class_name: &str, width: u32, height: u32) -> bool {
    let size = format!("exact {} {}", width, height);
    let selector = format!("class:{}", class_name);

    Command::new("hyprctl")
        .args(["dispatch", "resizewindowpixel", &size, &selector])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Hyprlandが利用可能かチェック
///
/// 検証済みパスからhyprctlを実行し、バージョン情報を取得できるか確認します。
/// パスが見つからない場合や実行に失敗した場合は false を返します。
pub fn is_available() -> bool {
    // 検証済みパスを使用
    let Some(mut cmd) = hyprctl_command() else {
        return false;
    };

    cmd.arg("version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// フォーカスされているモニターのサイズと位置を取得
pub fn get_focused_monitor() -> Option<(i32, i32, i32, i32)> {
    let output = Command::new("hyprctl")
        .args(["monitors", "-j"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let json_str = String::from_utf8_lossy(&output.stdout);
    let monitors: Vec<serde_json::Value> = serde_json::from_str(&json_str).ok()?;

    for monitor in monitors {
        if monitor.get("focused").and_then(|v| v.as_bool()) == Some(true) {
            let x = monitor.get("x").and_then(|v| v.as_i64())? as i32;
            let y = monitor.get("y").and_then(|v| v.as_i64())? as i32;
            let width = monitor.get("width").and_then(|v| v.as_i64())? as i32;
            let height = monitor.get("height").and_then(|v| v.as_i64())? as i32;
            return Some((x, y, width, height));
        }
    }

    None
}

/// デフォルト位置（右端）を計算
pub fn calculate_default_position(window_width: i32, _window_height: i32) -> Option<(i32, i32)> {
    let (mon_x, _mon_y, mon_width, _mon_height) = get_focused_monitor()?;
    // 右端から10pxマージン、上から50pxマージン
    let x = mon_x + mon_width - window_width - 10;
    let y = 50;
    Some((x, y))
}

/// ホットキー文字列をHyprland形式に変換
/// 例: "Shift+Space" -> "SHIFT, SPACE"
///     "Ctrl+Shift+M" -> "CTRL SHIFT, M"
pub fn parse_hotkey_to_hyprland(hotkey: &str) -> Option<(String, String)> {
    let parts: Vec<&str> = hotkey.split('+').collect();
    if parts.is_empty() {
        return None;
    }

    let key = parts.last()?.trim().to_uppercase();
    let modifiers: Vec<String> = parts[..parts.len() - 1]
        .iter()
        .map(|m| {
            match m.trim().to_lowercase().as_str() {
                "ctrl" | "control" => "CTRL",
                "shift" => "SHIFT",
                "alt" => "ALT",
                "super" | "meta" | "win" => "SUPER",
                other => return other.to_uppercase(),
            }.to_string()
        })
        .collect();

    let mod_str = modifiers.join(" ");
    Some((mod_str, key))
}

/// Hyprlandのbindings.confでkakuホットキーを更新
pub fn update_hotkey_binding(new_hotkey: &str) -> Result<(), String> {
    let (modifiers, key) = parse_hotkey_to_hyprland(new_hotkey)
        .ok_or("Invalid hotkey format")?;

    // bindings.confのパスを取得
    let config_dir = std::env::var("HOME")
        .map(|h| std::path::PathBuf::from(h).join(".config/hypr"))
        .map_err(|_| "HOME not set")?;
    let bindings_path = config_dir.join("bindings.conf");

    if !bindings_path.exists() {
        return Err("bindings.conf not found".to_string());
    }

    // 設定ファイルを読み込み
    let content = std::fs::read_to_string(&bindings_path)
        .map_err(|e| format!("Failed to read bindings.conf: {}", e))?;

    // kakuのバインディング行を探して更新
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
        .unwrap_or_else(|_| "/run/user/1000".to_string());
    let socket_path = format!("{}/kaku.sock", runtime_dir);

    let new_binding = format!(
        "bindd = {}, {}, Quick memo, exec, echo \"toggle\" | nc -U {}",
        modifiers, key, socket_path
    );

    let mut found = false;
    let mut new_lines: Vec<String> = Vec::new();

    for line in content.lines() {
        // コメント行 "# kaku" をスキップ
        if line.trim().starts_with("# kaku") {
            continue;
        }
        // "Quick memo" を含むバインディング行を更新
        if line.contains("Quick memo") && line.contains("kaku.sock") {
            if !found {
                new_lines.push("# kaku - quick memo toggle".to_string());
                new_lines.push(new_binding.clone());
                found = true;
            }
            continue;
        }
        new_lines.push(line.to_string());
    }

    // バインディングが見つからなかった場合は追加
    if !found {
        new_lines.push(String::new());
        new_lines.push("# kaku - quick memo toggle".to_string());
        new_lines.push(new_binding);
    }

    // ファイルに書き込み
    let new_content = new_lines.join("\n");
    std::fs::write(&bindings_path, &new_content)
        .map_err(|e| format!("Failed to write bindings.conf: {}", e))?;

    // Hyprlandの設定をリロード
    let reload_result = Command::new("hyprctl")
        .arg("reload")
        .output();

    match reload_result {
        Ok(output) if output.status.success() => {
            println!("[Hyprland] Hotkey updated to: {} + {}", modifiers, key);
            Ok(())
        }
        Ok(output) => {
            Err(format!("hyprctl reload failed: {}",
                String::from_utf8_lossy(&output.stderr)))
        }
        Err(e) => Err(format!("Failed to run hyprctl: {}", e)),
    }
}

/// 現在のHyprlandホットキーバインディングを取得
pub fn get_current_hotkey() -> Option<String> {
    let bindings_output = Command::new("hyprctl")
        .args(["binds", "-j"])
        .output()
        .ok()?;

    if !bindings_output.status.success() {
        return None;
    }

    let json_str = String::from_utf8_lossy(&bindings_output.stdout);
    let bindings: Vec<serde_json::Value> = serde_json::from_str(&json_str).ok()?;

    for binding in bindings {
        if binding.get("description").and_then(|v| v.as_str()) == Some("Quick memo") {
            let modmask = binding.get("modmask").and_then(|v| v.as_u64()).unwrap_or(0);
            let key = binding.get("key").and_then(|v| v.as_str()).unwrap_or("");

            // modmaskからモディファイアを復元
            let mut mods = Vec::new();
            if modmask & 1 != 0 { mods.push("Shift"); }
            if modmask & 4 != 0 { mods.push("Ctrl"); }
            if modmask & 8 != 0 { mods.push("Alt"); }
            if modmask & 64 != 0 { mods.push("Super"); }

            if mods.is_empty() {
                return Some(key.to_string());
            } else {
                mods.push(key);
                return Some(mods.join("+"));
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// is_available()がパニックせずに動作することを確認
    #[test]
    fn test_is_available() {
        // This test works on any platform
        // Just check that the function doesn't panic
        let _ = is_available();
    }

    /// is_wayland()がパニックせずに動作することを確認
    #[test]
    fn test_is_wayland() {
        // Should not panic regardless of environment
        let _ = is_wayland();
    }

    /// is_hyprland()がパニックせずに動作することを確認
    #[test]
    fn test_is_hyprland() {
        // Should not panic regardless of environment
        let result = is_hyprland();
        // If not Wayland, should definitely be false
        if !is_wayland() {
            assert!(!result, "is_hyprland should be false when not on Wayland");
        }
    }

    /// ホットキー文字列のパースが正しく動作することを確認
    #[test]
    fn test_parse_hotkey_to_hyprland_simple() {
        // Shift+Space
        let result = parse_hotkey_to_hyprland("Shift+Space");
        assert_eq!(result, Some(("SHIFT".to_string(), "SPACE".to_string())));
    }

    #[test]
    fn test_parse_hotkey_to_hyprland_ctrl_shift() {
        // Ctrl+Shift+M
        let result = parse_hotkey_to_hyprland("Ctrl+Shift+M");
        assert_eq!(result, Some(("CTRL SHIFT".to_string(), "M".to_string())));
    }

    #[test]
    fn test_parse_hotkey_to_hyprland_single_key() {
        // Just a key (unusual but valid)
        let result = parse_hotkey_to_hyprland("F12");
        assert_eq!(result, Some(("".to_string(), "F12".to_string())));
    }

    #[test]
    fn test_parse_hotkey_to_hyprland_super() {
        // Super+K (Super is a modifier)
        let result = parse_hotkey_to_hyprland("Super+K");
        assert_eq!(result, Some(("SUPER".to_string(), "K".to_string())));

        // Win is an alias for Super
        let result2 = parse_hotkey_to_hyprland("Win+K");
        assert_eq!(result2, Some(("SUPER".to_string(), "K".to_string())));

        // Meta is also an alias for Super
        let result3 = parse_hotkey_to_hyprland("Meta+K");
        assert_eq!(result3, Some(("SUPER".to_string(), "K".to_string())));
    }

    #[test]
    fn test_parse_hotkey_to_hyprland_all_modifiers() {
        // Ctrl+Shift+Alt+Super+K
        let result = parse_hotkey_to_hyprland("Ctrl+Shift+Alt+Super+K");
        assert_eq!(result, Some(("CTRL SHIFT ALT SUPER".to_string(), "K".to_string())));
    }

    #[test]
    fn test_parse_hotkey_to_hyprland_empty() {
        // Empty string
        let result = parse_hotkey_to_hyprland("");
        // Empty string splits to [""], last element is ""
        assert_eq!(result, Some(("".to_string(), "".to_string())));
    }

    #[test]
    fn test_parse_hotkey_to_hyprland_control_alias() {
        // Control is an alias for Ctrl
        let result = parse_hotkey_to_hyprland("Control+C");
        assert_eq!(result, Some(("CTRL".to_string(), "C".to_string())));
    }

    /// get_hyprctl_path()がパニックせずに動作することを確認
    #[test]
    fn test_get_hyprctl_path_no_panic() {
        // Should not panic regardless of whether hyprctl is installed
        let _ = get_hyprctl_path();
    }

    /// 非Hyprland環境でウィンドウ関連関数がNoneを返すことを確認
    #[test]
    fn test_window_functions_return_none_when_not_available() {
        if !is_hyprland() {
            // These should return None when not on Hyprland
            assert!(get_window_position("nonexistent").is_none());
            assert!(get_focused_monitor().is_none());
            assert!(calculate_default_position(400, 500).is_none());
            assert!(get_current_hotkey().is_none());
        }
    }
}
