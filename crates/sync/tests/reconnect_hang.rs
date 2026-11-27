mod support;

use std::sync::{Arc, Mutex};
use std::time::Duration;

use anybot_foundation::Clock;
use anybot_sync::heartbeat_keepalive::RequestActivity;
use anybot_sync::reconnect_http::SyncHttp;
use anybot_sync::reconnect_pacing::ReconnectPacer;
use anybot_sync::reconnect_random::SeededRandom;
use anybot_sync::sse_session::{SseEnd, SseTarget, run_sse_session};
use support::{end_of, http_on, settle, start_session, target, walk};
use test_harness::netctl::{net_attach, net_hang_connect, net_start};
use test_harness::origin::start_origin;
use test_harness::{FakeClock, advance_clock, timeout_real};
use tokio::sync::mpsc;

use anybot_app_test as _;
use anybot_domain as _;
use anybot_foundation as _;
use anybot_protocol as _;
use anybot_server_test as _;
use anybot_store_client as _;
use bytes as _;
use getrandom as _;
use proptest as _;
use reqwest as _;
use serde_json as _;
use tokio as _;
use tower as _;
use url as _;

const CLIENT_START_MS: u64 = 40_000_000;
const UNREACHABLE: &str = "https://203.0.113.5:8443";

struct Attempt {
    started: u64,
    failed: u64,
}

async fn hanging_link(clock: &Arc<FakeClock>) -> Result<Arc<SyncHttp>, String> {
    let origin = timeout_real(start_origin(), Duration::from_secs(20))
        .await
        .map_err(|_| "origin start".to_owned())??;
    let (ctl, port) = net_start().await?;
    net_attach(&ctl, port, origin).await?;
    net_hang_connect(&ctl, port).await?;
    http_on(clock, Some(&format!("http://127.0.0.1:{port}")))
}

fn unreachable_target() -> SseTarget {
    target(UNREACHABLE, "any")
}

#[tokio::test]
async fn sse_session_to_an_address_that_never_answers_fails_in_the_connect_phase_at_5_seconds()
-> Result<(), String> {
    let clock = FakeClock::new(CLIENT_START_MS);
    let http = hanging_link(&clock).await?;
    let mut running = start_session(&http, &clock, unreachable_target());
    settle().await;
    let _woken = advance_clock(&clock, Duration::from_millis(4_999));
    settle().await;
    assert!(!running.handle.is_finished(), "inside the 5 seconds");
    let _woken = advance_clock(&clock, Duration::from_millis(1));
    let silent = running.signals.try_recv().is_err();
    assert!(silent, "nothing is reported before the connect phase ends");
    let ended = end_of(running).await?;
    assert_eq!(ended, SseEnd::ConnectTimeout);
    Ok(())
}

async fn drive(
    http: Arc<SyncHttp>,
    clock: Arc<FakeClock>,
    pacer: Arc<ReconnectPacer>,
    log: Arc<Mutex<Vec<Attempt>>>,
) {
    let shared: Arc<dyn Clock> = Arc::<FakeClock>::clone(&clock);
    let aim = unreachable_target();
    loop {
        pacer.wait_turn().await;
        let started = shared.now_ms().millis();
        let activity = RequestActivity::new(Arc::clone(&shared));
        let (sender, _signals) = mpsc::channel(4);
        let _end = run_sse_session(&http, Arc::clone(&shared), &aim, &activity, &sender).await;
        let failed = shared.now_ms().millis();
        let _delay = pacer.failed();
        if let Ok(mut rows) = log.lock() {
            rows.push(Attempt { started, failed });
        }
    }
}

#[tokio::test]
async fn reconnect_over_an_address_that_never_answers_fails_in_5_seconds_then_backs_off_1_2_4_8_16_30_30()
-> Result<(), String> {
    let clock = FakeClock::new(CLIENT_START_MS);
    let http = hanging_link(&clock).await?;
    let shared: Arc<dyn Clock> = Arc::<FakeClock>::clone(&clock);
    let pacer = Arc::new(ReconnectPacer::new(shared, Arc::new(SeededRandom::new(11))));
    let log = Arc::new(Mutex::new(Vec::new()));
    let driver = tokio::spawn(drive(http, Arc::clone(&clock), pacer, Arc::clone(&log)));
    for _round in 0..40 {
        walk(&clock, Duration::from_secs(5), Duration::from_millis(250)).await?;
        if log.lock().map_err(|e| e.to_string())?.len() >= 8 {
            break;
        }
    }
    driver.abort();
    let rows = log.lock().map_err(|e| e.to_string())?;
    assert!(rows.len() >= 8, "attempts: {}", rows.len());
    let bases = [1_000u64, 2_000, 4_000, 8_000, 16_000, 30_000, 30_000];
    for (index, attempt) in rows.iter().enumerate() {
        let took = attempt.failed - attempt.started;
        assert!(
            (5_000..=5_250).contains(&took),
            "attempt {index} took {took}"
        );
    }
    for (index, (pair, base)) in rows.windows(2).zip(bases.iter()).enumerate() {
        let [before, after] = pair else {
            return Err("window of two expected".to_owned());
        };
        let gap = after.started - before.failed;
        assert!(
            (base + 500..=base + 2_250).contains(&gap),
            "gap {index} was {gap} ms, base {base}"
        );
    }
    Ok(())
}
