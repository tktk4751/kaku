use crate::domain::DomainEvent;
use std::sync::Arc;

/// イベントハンドラ型
pub type EventHandler = Arc<dyn Fn(&DomainEvent) + Send + Sync>;

/// イベントバス抽象化（Observer/EventBusパターン）
pub trait EventBus: Send + Sync {
    /// イベントを発行
    fn emit(&self, event: DomainEvent);

    /// イベントハンドラを登録
    fn subscribe(&self, event_name: &str, handler: EventHandler) -> SubscriptionId;

    /// イベントハンドラを解除
    fn unsubscribe(&self, id: SubscriptionId);
}

/// サブスクリプションID（購読解除用）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SubscriptionId(pub u64);
