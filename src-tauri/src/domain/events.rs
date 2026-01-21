use serde::{Deserialize, Serialize};

/// ドメインイベント（Observer/EventBusパターン）
///
/// # 現在の使用状況
///
/// ## 使用中
/// - `NoteCreated`: note_service.rs で発火
/// - `NoteUpdated`: 将来のリアルタイム同期用（テストで使用）
/// - `NoteDeleted`: note_service.rs で発火
/// - `NoteLoaded`: note_service.rs で発火
/// - `SaveCompleted`: note_service.rs で発火
/// - `SettingsChanged`: settings_service.rs で発火
///
/// ## 将来の拡張用（現在未使用）
/// - `SaveRequested`: 保存キューイング実装時
/// - `SaveFailed`: エラー通知UI実装時
/// - `WindowShown`/`WindowHidden`: フロントエンド連携時
/// - `AppQuitting`: 終了時クリーンアップ処理
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)] // 将来の拡張用イベントを含む
pub enum DomainEvent {
    /// メモが作成された
    NoteCreated { uid: String },
    /// メモが更新された
    NoteUpdated { uid: String },
    /// メモが削除された
    NoteDeleted { uid: String },
    /// メモがロードされた
    NoteLoaded { uid: String },
    /// 保存がリクエストされた（将来の保存キューイング用）
    SaveRequested { uid: String },
    /// 保存が完了した
    SaveCompleted { uid: String },
    /// 保存が失敗した（将来のエラー通知UI用）
    SaveFailed { uid: String, error: String },
    /// 設定が変更された
    SettingsChanged,
    /// ウィンドウが表示された（将来のフロントエンド連携用）
    WindowShown,
    /// ウィンドウが非表示になった（将来のフロントエンド連携用）
    WindowHidden,
    /// アプリケーションが終了する（将来の終了時クリーンアップ用）
    AppQuitting,
}

impl DomainEvent {
    /// イベント名を取得
    pub fn name(&self) -> &'static str {
        match self {
            DomainEvent::NoteCreated { .. } => "note:created",
            DomainEvent::NoteUpdated { .. } => "note:updated",
            DomainEvent::NoteDeleted { .. } => "note:deleted",
            DomainEvent::NoteLoaded { .. } => "note:loaded",
            DomainEvent::SaveRequested { .. } => "save:requested",
            DomainEvent::SaveCompleted { .. } => "save:completed",
            DomainEvent::SaveFailed { .. } => "save:failed",
            DomainEvent::SettingsChanged => "settings:changed",
            DomainEvent::WindowShown => "window:shown",
            DomainEvent::WindowHidden => "window:hidden",
            DomainEvent::AppQuitting => "app:quitting",
        }
    }
}
