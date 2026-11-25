use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use tokio::sync::watch;

fn slot() -> &'static Mutex<Option<watch::Receiver<bool>>> {
    static SLOT: OnceLock<Mutex<Option<watch::Receiver<bool>>>> = OnceLock::new();
    SLOT.get_or_init(|| Mutex::new(None))
}

#[must_use]
pub fn arm() -> watch::Sender<bool> {
    let (tx, rx) = watch::channel(false);
    if let Ok(mut guard) = slot().lock() {
        *guard = Some(rx);
    }
    tx
}

pub(crate) async fn wait_release() {
    let rx = slot().lock().ok().and_then(|mut guard| guard.take());
    let Some(mut rx) = rx else {
        tokio::time::sleep(Duration::from_secs(3600)).await;
        return;
    };
    if *rx.borrow() {
        return;
    }
    let _changed = rx.changed().await;
}
