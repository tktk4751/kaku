use crate::domain::DomainEvent;
use crate::traits::{EventBus, EventHandler, SubscriptionId};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

/// EventBusの実装（Observer/Pub-Subパターン）
pub struct EventBusImpl {
    /// イベント名 → ハンドラのマップ
    handlers: RwLock<HashMap<String, Vec<(SubscriptionId, EventHandler)>>>,
    /// 次のサブスクリプションID
    next_id: AtomicU64,
}

impl EventBusImpl {
    pub fn new() -> Self {
        Self {
            handlers: RwLock::new(HashMap::new()),
            next_id: AtomicU64::new(1),
        }
    }
}

impl Default for EventBusImpl {
    fn default() -> Self {
        Self::new()
    }
}

impl EventBus for EventBusImpl {
    fn emit(&self, event: DomainEvent) {
        let event_name = event.name().to_string();
        let handlers = self.handlers.read();

        // 特定イベントのハンドラを実行
        if let Some(event_handlers) = handlers.get(&event_name) {
            for (_, handler) in event_handlers {
                handler(&event);
            }
        }

        // ワイルドカード("*")ハンドラも実行
        if let Some(wildcard_handlers) = handlers.get("*") {
            for (_, handler) in wildcard_handlers {
                handler(&event);
            }
        }
    }

    fn subscribe(&self, event_name: &str, handler: EventHandler) -> SubscriptionId {
        let id = SubscriptionId(self.next_id.fetch_add(1, Ordering::SeqCst));
        let mut handlers = self.handlers.write();

        handlers
            .entry(event_name.to_string())
            .or_default()
            .push((id, handler));

        id
    }

    fn unsubscribe(&self, id: SubscriptionId) {
        let mut handlers = self.handlers.write();

        for (_, event_handlers) in handlers.iter_mut() {
            event_handlers.retain(|(sub_id, _)| *sub_id != id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;
    use std::sync::Arc;

    #[test]
    fn test_emit_and_subscribe() {
        let bus = EventBusImpl::new();
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        bus.subscribe(
            "note:created",
            Arc::new(move |_| {
                counter_clone.fetch_add(1, Ordering::SeqCst);
            }),
        );

        bus.emit(DomainEvent::NoteCreated {
            uid: "test".to_string(),
        });

        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_wildcard_subscription() {
        let bus = EventBusImpl::new();
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        bus.subscribe(
            "*",
            Arc::new(move |_| {
                counter_clone.fetch_add(1, Ordering::SeqCst);
            }),
        );

        bus.emit(DomainEvent::NoteCreated {
            uid: "test".to_string(),
        });
        bus.emit(DomainEvent::NoteUpdated {
            uid: "test".to_string(),
        });

        assert_eq!(counter.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn test_unsubscribe() {
        let bus = EventBusImpl::new();
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let id = bus.subscribe(
            "note:created",
            Arc::new(move |_| {
                counter_clone.fetch_add(1, Ordering::SeqCst);
            }),
        );

        bus.emit(DomainEvent::NoteCreated {
            uid: "test".to_string(),
        });
        assert_eq!(counter.load(Ordering::SeqCst), 1);

        bus.unsubscribe(id);

        bus.emit(DomainEvent::NoteCreated {
            uid: "test".to_string(),
        });
        assert_eq!(counter.load(Ordering::SeqCst), 1); // 変わらない
    }
}
