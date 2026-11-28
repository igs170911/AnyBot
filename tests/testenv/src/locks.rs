use std::collections::HashSet;
use std::sync::Mutex;
use std::time::Duration;

use tokio::sync::Notify;

use test_harness::realtime::timeout_real;

pub const LOCK_WAIT: Duration = Duration::from_secs(60);

#[derive(Default)]
pub struct LockTable {
    held: Mutex<HashSet<String>>,
    freed: Notify,
}

impl LockTable {
    fn try_take(&self, name: &str) -> bool {
        let Ok(mut held) = self.held.lock() else {
            return false;
        };
        held.insert(name.to_owned())
    }

    pub async fn acquire(&self, name: &str, wait: Duration) -> Result<(), String> {
        let attempt = async {
            loop {
                let notified = self.freed.notified();
                if self.try_take(name) {
                    return;
                }
                notified.await;
            }
        };
        timeout_real(attempt, wait)
            .await
            .map_err(|_e| format!("lock {name} was not released within {wait:?}"))
    }

    pub fn release(&self, name: &str) -> bool {
        let Ok(mut held) = self.held.lock() else {
            return false;
        };
        let was_held = held.remove(name);
        drop(held);
        self.freed.notify_waiters();
        was_held
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    const SHORT: Duration = Duration::from_millis(200);
    const LONG: Duration = Duration::from_secs(10);

    #[tokio::test]
    async fn control_lock_serializes_holders() -> Result<(), String> {
        let table = Arc::new(LockTable::default());
        table.acquire("fixed-account-bob", LONG).await?;
        let early = table.acquire("fixed-account-bob", SHORT).await;
        assert!(
            early.is_err(),
            "second holder must wait while the first holds"
        );
        let waiter = {
            let table = Arc::clone(&table);
            tokio::spawn(async move { table.acquire("fixed-account-bob", LONG).await })
        };
        test_harness::sleep_real(Duration::from_millis(50)).await;
        assert!(table.release("fixed-account-bob"));
        let got = timeout_real(waiter, LONG)
            .await?
            .map_err(|e| e.to_string())?;
        got?;
        assert!(table.release("fixed-account-bob"));
        assert!(!table.release("fixed-account-bob"));
        Ok(())
    }

    #[tokio::test]
    async fn different_lock_names_do_not_block_each_other() -> Result<(), String> {
        let table = LockTable::default();
        table.acquire("clock", LONG).await?;
        table.acquire("other", SHORT).await?;
        Ok(())
    }
}
