// 設定リポジトリトレイト
//
// SOLID: Dependency Inversion Principle
// SettingsService は具象実装ではなくこのトレイトに依存する

use crate::domain::{Settings, SettingsError};

/// 設定の永続化を担当するリポジトリ
pub trait SettingsRepository: Send + Sync {
    /// 設定をロード
    fn load(&self) -> Result<Settings, SettingsError>;

    /// 設定を保存
    fn save(&self, settings: &Settings) -> Result<(), SettingsError>;
}
