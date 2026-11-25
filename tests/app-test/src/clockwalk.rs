use std::sync::Arc;
use std::time::Duration;

use tokio::task::JoinHandle;

use test_harness::FakeClock;

pub const STEP_FAKE: Duration = Duration::from_millis(100);
pub const STEP_REAL: Duration = Duration::from_millis(10);

pub struct ClockWalker {
    task: Option<JoinHandle<()>>,
}

impl ClockWalker {
    pub fn start(clock: &Arc<FakeClock>) -> Self {
        let owned = Arc::clone(clock);
        ClockWalker {
            task: Some(tokio::spawn(async move {
                loop {
                    test_harness::sleep_real(STEP_REAL).await;
                    let _woken = owned.advance(STEP_FAKE);
                }
            })),
        }
    }

    pub async fn stop(&mut self) -> Result<(), String> {
        let Some(task) = self.task.take() else {
            return Ok(());
        };
        task.abort();
        match task.await {
            Ok(()) => Ok(()),
            Err(e) if e.is_cancelled() => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    }
}

impl Drop for ClockWalker {
    fn drop(&mut self) {
        if let Some(task) = &self.task {
            task.abort();
        }
    }
}
