use std::sync::Arc;
use std::time::Duration;

use crate::catalog::LIST_WAIT_SECS;
use crate::catalog_fetch::fetch_models;
use crate::prop_run::{hi, model_ref};
use crate::scripted::{ScriptPoster, held, ok_text, script_http};
use crate::session::Session;
use crate::types::{Audience, ProviderKind, WireFormat};

fn runtime() -> Result<tokio::runtime::Runtime, String> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|err| err.to_string())
}

async fn release_after(hold: &Arc<tokio::sync::Notify>, clock: &Arc<ManualClock>, secs: u64) {
    tokio::task::yield_now().await;
    for _ in 0..secs {
        clock.advance(1_000);
        tokio::task::yield_now().await;
    }
    hold.notify_waiters();
    hold.notify_one();
}

struct Sleeper {
    deadline: u64,
    wake: Arc<tokio::sync::Notify>,
}

struct ManualClock {
    now: std::sync::Mutex<u64>,
    sleepers: std::sync::Mutex<Vec<Sleeper>>,
}

impl ManualClock {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            now: std::sync::Mutex::new(0),
            sleepers: std::sync::Mutex::new(Vec::new()),
        })
    }

    fn now(&self) -> u64 {
        match self.now.lock() {
            Ok(now) => *now,
            Err(poisoned) => *poisoned.into_inner(),
        }
    }

    fn advance(self: &Arc<Self>, ms: u64) {
        let next = self.now().saturating_add(ms);
        match self.now.lock() {
            Ok(mut now) => *now = next,
            Err(poisoned) => *poisoned.into_inner() = next,
        }
        let due = match self.sleepers.lock() {
            Ok(mut sleepers) => drain_due(&mut sleepers, next),
            Err(poisoned) => drain_due(&mut poisoned.into_inner(), next),
        };
        for sleeper in due {
            sleeper.wake.notify_one();
        }
    }
}

fn drain_due(sleepers: &mut Vec<Sleeper>, now: u64) -> Vec<Sleeper> {
    let due = sleepers
        .iter()
        .filter(|sleeper| sleeper.deadline <= now)
        .count();
    sleepers.drain(..due).collect()
}

impl anybot_foundation::clock::Clock for ManualClock {
    fn now_ms(&self) -> anybot_foundation::clock::TimestampMs {
        anybot_foundation::clock::TimestampMs::from_millis(self.now())
    }

    fn sleep(&self, span: Duration) -> anybot_foundation::clock::PinBoxFuture<'static, ()> {
        let add = u64::try_from(span.as_millis()).unwrap_or(u64::MAX);
        let deadline = self.now().saturating_add(add);
        let wake = Arc::new(tokio::sync::Notify::new());
        let sleeper = Sleeper {
            deadline,
            wake: Arc::clone(&wake),
        };
        match self.sleepers.lock() {
            Ok(mut sleepers) => sleepers.push(sleeper),
            Err(poisoned) => poisoned.into_inner().push(sleeper),
        }
        Box::pin(async move {
            wake.notified().await;
        })
    }

    fn sleep_until(
        &self,
        deadline: anybot_foundation::clock::TimestampMs,
    ) -> anybot_foundation::clock::PinBoxFuture<'static, ()> {
        let span = deadline.millis().saturating_sub(self.now());
        self.sleep(Duration::from_millis(span))
    }
}

fn session_of(poster: Arc<ScriptPoster>, clock: Arc<ManualClock>) -> Session {
    Session::new(
        crate::client::Gateway::from_wire(
            poster,
            Arc::clone(&clock) as Arc<dyn anybot_foundation::clock::Clock>,
        ),
        crate::egress::DataPolicy::LocalOnly,
        clock,
        0,
    )
}

#[test]
fn slow_non_stream_answer_succeeds_after_connect() -> Result<(), String> {
    let clock = ManualClock::new();
    let hold = Arc::new(tokio::sync::Notify::new());
    let body = ok_text("late").body;
    let poster = ScriptPoster::new(vec![held(script_http(200, &body), Arc::clone(&hold))]);
    let primary = model_ref(crate::prop_run::custom("m", WireFormat::OpenAiCompatible));
    let request = hi();
    let mut session = session_of(poster, Arc::clone(&clock));
    let built = runtime()?;
    let outcome = built.block_on(async {
        let mut plan = crate::session::ask_plan(&primary, &[], &request, Audience::Watching);
        let pending = session.ask(&mut plan);
        tokio::pin!(pending);
        let release = release_after(&hold, &clock, 20);
        tokio::select! {
            biased;
            outcome = &mut pending => outcome,
            () = release => pending.await,
        }
    });
    assert!(outcome.is_ok(), "{outcome:?}");
    Ok(())
}

#[test]
fn model_list_waits_the_catalog_limit() -> Result<(), String> {
    let clock = ManualClock::new();
    let hold = Arc::new(tokio::sync::Notify::new());
    let poster = ScriptPoster::new(vec![held(
        script_http(200, "{\"data\":[{\"id\":\"m\"}]}"),
        Arc::clone(&hold),
    )]);
    let endpoint =
        crate::prop_run::endpoint(ProviderKind::Custom, "http://127.0.0.1:9/v1", "", "m");
    let gateway = crate::client::Gateway::from_wire(
        poster,
        Arc::clone(&clock) as Arc<dyn anybot_foundation::clock::Clock>,
    );
    let built = runtime()?;
    let outcome = built.block_on(async {
        let pending = fetch_models(&gateway, &endpoint);
        tokio::pin!(pending);
        let release = release_after(&hold, &clock, LIST_WAIT_SECS.saturating_sub(1));
        tokio::select! {
            biased;
            outcome = &mut pending => outcome,
            () = release => pending.await,
        }
    });
    assert!(
        matches!(outcome, Ok(crate::catalog_fetch::ListOutcome::Names(_))),
        "{outcome:?}"
    );
    Ok(())
}
