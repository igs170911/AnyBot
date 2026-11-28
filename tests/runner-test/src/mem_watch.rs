use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use anybot_foundation::TaskId;
use anybot_runner::{TaskWatch, WatchFuture};
use tokio::sync::watch;

#[derive(Default)]
pub struct MemoryWatch {
    channels: Mutex<BTreeMap<TaskId, watch::Sender<u64>>>,
}

impl MemoryWatch {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn notify(&self, task: TaskId) {
        let sender = self.sender_of(task);
        sender.send_modify(|v| {
            *v = v.saturating_add(1);
        });
    }

    fn subscribe(&self, task: TaskId) -> watch::Receiver<u64> {
        self.sender_of(task).subscribe()
    }

    fn sender_of(&self, task: TaskId) -> watch::Sender<u64> {
        let Ok(mut guard) = self.channels.lock() else {
            let (tx, _rx) = watch::channel(0);
            return tx;
        };
        guard
            .entry(task)
            .or_insert_with(|| {
                let (tx, _rx) = watch::channel(0);
                tx
            })
            .clone()
    }
}

impl TaskWatch for MemoryWatch {
    fn changed(&self, task: TaskId) -> WatchFuture<'_> {
        let mut rx = self.subscribe(task);
        Box::pin(async move { if let Err(_closed) = rx.changed().await {} })
    }
}

#[must_use]
pub fn share_watch(watch: &Arc<MemoryWatch>) -> Arc<dyn TaskWatch> {
    let shared: Arc<MemoryWatch> = Arc::clone(watch);
    shared
}
