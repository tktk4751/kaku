use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// ウィンドウジオメトリ設定
///
/// ポップアップスタイルのウィンドウ用。最大化は非サポート。
///
/// # フィールド
///
/// - `x`, `y`: ウィンドウ位置（-1 = 中央配置）
/// - `width`, `height`: ウィンドウサイズ
/// - `is_maximized`: **非推奨** - 後方互換性のみ、常に無視される
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WindowGeometry {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    /// **非推奨**: このフィールドは後方互換性のために存在しますが、
    /// アプリケーションでは使用されません。
    ///
    /// # 理由
    ///
    /// kakuはポップアップスタイルのメモアプリであり、
    /// 最大化はUXとして適切ではないため、この機能は削除されました。
    /// 既存の設定ファイルとの互換性を保つため、フィールドは残されていますが、
    /// 読み込み時のみ受け付け、保存時には出力されません。
    ///
    /// # 移行
    ///
    /// このフィールドは将来のバージョンで完全に削除される予定です。
    /// 設定ファイルからこのフィールドを手動で削除しても問題ありません。
    #[serde(default, skip_serializing)]
    #[deprecated(since = "0.2.0", note = "最大化機能は削除されました。このフィールドは無視されます。")]
    pub is_maximized: bool,
}

impl Default for WindowGeometry {
    #[allow(deprecated)]
    fn default() -> Self {
        Self {
            x: -1, // -1 = 中央配置を示す特別な値
            y: -1,
            width: 400,
            height: 500,
            is_maximized: false,
        }
    }
}

/// エディタ設定
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EditorSettings {
    pub font_family: String,
    pub font_size: u32,
    pub line_height: f32,
    #[serde(default = "default_show_line_numbers")]
    pub show_line_numbers: bool,
}

fn default_show_line_numbers() -> bool {
    true
}

impl Default for EditorSettings {
    fn default() -> Self {
        Self {
            font_family: "system-ui".to_string(),
            font_size: 14,
            line_height: 1.6,
            show_line_numbers: true,
        }
    }
}

/// カラーテーマ名
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum ThemeName {
    TokyoNight,
    Kanagawa,
    Monokai,
    Gruvbox,
    Dracula,
    Catppuccin,
    Synthwave,
}

impl Default for ThemeName {
    fn default() -> Self {
        Self::TokyoNight
    }
}

/// テーマモード（ライト/ダーク）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum ThemeMode {
    Light,
    Dark,
}

impl Default for ThemeMode {
    fn default() -> Self {
        Self::Dark
    }
}

/// 自動保存設定
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AutosaveSettings {
    pub enabled: bool,
    pub delay_ms: u64,
}

impl Default for AutosaveSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            delay_ms: 2000,
        }
    }
}

/// ショートカットキー設定
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ShortcutSettings {
    pub new_note: String,
    pub toggle_sidebar: String,
    pub open_settings: String,
}

impl Default for ShortcutSettings {
    fn default() -> Self {
        Self {
            new_note: "Ctrl+N".to_string(),
            toggle_sidebar: "Ctrl+M".to_string(),
            open_settings: "Ctrl+,".to_string(),
        }
    }
}

/// アプリケーション設定
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Settings {
    pub window: WindowGeometry,
    pub storage_directory: PathBuf,
    pub editor: EditorSettings,
    pub theme: ThemeName,
    #[serde(default)]
    pub theme_mode: ThemeMode,
    pub hotkey: String,
    #[serde(default)]
    pub shortcuts: ShortcutSettings,
    pub autosave: AutosaveSettings,
    pub restore_last_note: bool,
    #[serde(default)]
    pub last_note_uid: Option<String>,
}

impl Settings {
    /// デフォルト保存ディレクトリを取得
    fn default_storage_directory() -> PathBuf {
        directories::UserDirs::new()
            .and_then(|dirs| dirs.document_dir().map(|d| d.to_path_buf()))
            .unwrap_or_else(|| PathBuf::from("."))
            .join("kaku")
    }

    /// 設定ファイルパスを取得
    pub fn config_path() -> PathBuf {
        directories::ProjectDirs::from("dev", "z", "kaku")
            .map(|dirs| dirs.config_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from(".config/kaku"))
            .join("config.toml")
    }

    /// TOMLファイルからロード
    pub fn load_from_file(path: &std::path::Path) -> Result<Self, SettingsError> {
        let content = std::fs::read_to_string(path)?;
        let settings: Settings = toml::from_str(&content)?;
        Ok(settings)
    }

    /// TOMLファイルに保存
    pub fn save_to_file(&self, path: &std::path::Path) -> Result<(), SettingsError> {
        let content = toml::to_string_pretty(self)?;

        // 親ディレクトリを作成
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // アトミック書き込み
        let temp_path = path.with_extension("toml.tmp");
        std::fs::write(&temp_path, &content)?;
        std::fs::rename(&temp_path, path)?;

        Ok(())
    }

    /// ウィンドウジオメトリを更新
    pub fn update_window_geometry(&mut self, geometry: WindowGeometry) {
        self.window = geometry;
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            window: WindowGeometry::default(),
            storage_directory: Self::default_storage_directory(),
            editor: EditorSettings::default(),
            theme: ThemeName::default(),
            theme_mode: ThemeMode::default(),
            hotkey: "Ctrl+Shift+Space".to_string(),
            shortcuts: ShortcutSettings::default(),
            autosave: AutosaveSettings::default(),
            restore_last_note: false,
            last_note_uid: None,
        }
    }
}

/// 設定関連エラー
#[derive(Debug, thiserror::Error)]
pub enum SettingsError {
    #[error("IOエラー: {0}")]
    Io(#[from] std::io::Error),
    #[error("TOMLパースエラー: {0}")]
    TomlParse(#[from] toml::de::Error),
    #[error("TOMLシリアライズエラー: {0}")]
    TomlSerialize(#[from] toml::ser::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_default_settings() {
        let settings = Settings::default();
        assert_eq!(settings.window.width, 400);
        assert_eq!(settings.window.height, 500);
        assert_eq!(settings.hotkey, "Ctrl+Shift+Space");
        assert!(settings.autosave.enabled);
    }

    #[test]
    fn test_settings_roundtrip() {
        let settings = Settings::default();

        let temp_file = NamedTempFile::new().unwrap();
        settings.save_to_file(temp_file.path()).unwrap();

        let loaded = Settings::load_from_file(temp_file.path()).unwrap();
        assert_eq!(settings, loaded);
    }
}
