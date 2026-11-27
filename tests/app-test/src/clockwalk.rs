use std::sync::Arc;
use std::time::Duration;

use tokio::task::JoinHandle;

use test_harness::FakeClock;

pub const STEP_FAKE: Duration = Duration::from_millis(1000);
pub const STEP_REAL: Duration = Duration::from_millis(10);

pub struct ClockWalker {
    clock: Arc<FakeClock>,
    task: Option<JoinHandle<()>>,
}

fn owned_of(walker: &ClockWalker) -> &Arc<FakeClock> {
    &walker.clock
}

impl ClockWalker {
    pub fn start(clock: &Arc<FakeClock>) -> Self {
        let owned = Arc::clone(clock);
        ClockWalker {
            clock: Arc::clone(&owned),
            task: Some(tokio::spawn(async move {
                loop {
                    test_harness::sleep_real(STEP_REAL).await;
                    let _woken = owned.advance(STEP_FAKE);
                }
            })),
        }
    }

    pub async fn advance(&self, span: Duration) -> Result<usize, String> {
        let millis =
            u64::try_from(span.as_millis()).map_err(|_| "advance span too large".to_owned())?;
        let step = u64::try_from(STEP_FAKE.as_millis()).map_err(|_| "step too large".to_owned())?;
        let rounds = millis
            .checked_div(step)
            .ok_or("advance span shorter than one step".to_owned())?;
        for _round in 0..rounds {
            test_harness::sleep_real(STEP_REAL).await;
            let _woken = owned_of(self).advance(STEP_FAKE);
        }
        usize::try_from(rounds).map_err(|_| "advance rounds too large".to_owned())
    }

    #[must_use]
    pub fn manual(clock: &Arc<FakeClock>) -> Self {
        ClockWalker {
            clock: Arc::clone(clock),
            task: None,
        }
    }

    pub async fn step(&mut self, span: Duration) -> Result<usize, String> {
        if self.task.is_some() {
            return Err("step needs a manual walker".to_owned());
        }
        let millis =
            u64::try_from(span.as_millis()).map_err(|_| "step span too large".to_owned())?;
        let step = u64::try_from(STEP_FAKE.as_millis()).map_err(|_| "step too large".to_owned())?;
        let rounds = millis
            .checked_div(step)
            .ok_or("step span shorter than one step".to_owned())?;
        let mut woken = 0usize;
        for _round in 0..rounds {
            test_harness::sleep_real(STEP_REAL).await;
            woken = woken.saturating_add(self.clock.advance(STEP_FAKE));
        }
        Ok(usize::try_from(rounds).map_err(|_| "step rounds too large".to_owned())?)
            .map(|count| count.saturating_add(woken.saturating_sub(count)))
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

pub const WAVE_FAKE_STEP: Duration = Duration::from_millis(500);
pub const WAVE_INSERT_STEP: Duration = Duration::from_millis(10);

pub async fn settle_handles<T>(
    clock: &Arc<FakeClock>,
    handles: &mut Vec<JoinHandle<T>>,
    fake_step: Duration,
    deadline: Duration,
) -> Result<Vec<T>, String>
where
    T: Send,
{
    let total = handles.len();
    let started = tokio::time::Instant::now();
    loop {
        let finished = handles.iter().filter(|handle| handle.is_finished()).count();
        if finished == total {
            return collect(handles).await;
        }
        if started.elapsed() >= deadline {
            let pending = total.saturating_sub(finished);
            return Err(format!("{pending} of {total} answers still pending"));
        }
        test_harness::sleep_real(STEP_REAL).await;
        let _advanced = clock.advance(fake_step);
    }
}

async fn collect<T>(handles: &mut Vec<JoinHandle<T>>) -> Result<Vec<T>, String>
where
    T: Send,
{
    let mut answers = Vec::new();
    for handle in handles.drain(..) {
        answers.push(handle.await.map_err(|e| e.to_string())?);
    }
    Ok(answers)
}
