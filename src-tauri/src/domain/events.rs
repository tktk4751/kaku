use serde::{Deserialize, Serialize};

/// ドメインイベント（Observer/EventBusパターン）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DomainEvent {
    /// メモが作成された
    NoteCreated { uid: String },
    /// メモが更新された
    NoteUpdated { uid: String },
    /// メモが削除された
    NoteDeleted { uid: String },
    /// メモがロードされた
    NoteLoaded { uid: String },
    /// 保存がリクエストされた
    SaveRequested { uid: String },
    /// 保存が完了した
    SaveCompleted { uid: String },
    /// 保存が失敗した
    SaveFailed { uid: String, error: String },
    /// 設定が変更された
    SettingsChanged,
    /// ウィンドウが表示された
    WindowShown,
    /// ウィンドウが非表示になった
    WindowHidden,
    /// アプリケーションが終了する
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
