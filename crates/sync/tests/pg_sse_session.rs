mod support;

use std::time::Duration;

use anybot_sync::sse_hint::Hint;
use anybot_sync::sse_session::{SseEnd, SseSignal};
use support::{append_one, end_of, http_on, next_signal, rig, settle, start_session, target, walk};
use test_harness::{FakeClock, advance_clock};

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

const CLIENT_START_MS: u64 = 9_000_000;

#[tokio::test]
async fn sse_session_starts_with_progress_then_reports_news_for_a_new_record() -> Result<(), String>
{
    let rig = rig("sync_sse_progress").await?;
    append_one(&rig, "k-sse-1", "dev-1").await?;
    append_one(&rig, "k-sse-2", "dev-2").await?;
    let clock = FakeClock::new(CLIENT_START_MS);
    let http = http_on(&clock, None)?;
    let mut running = start_session(&http, &clock, target(&rig.base, &rig.token));
    assert_eq!(next_signal(&mut running).await?, SseSignal::Connected);
    let SseSignal::Hint(Hint::Progress { streams }) = next_signal(&mut running).await? else {
        return Err("the first hint must be the progress".to_owned());
    };
    assert!(
        streams
            .iter()
            .any(|mark| mark.stream == "account/alice" && mark.seq == 2),
        "progress: {streams:?}"
    );
    append_one(&rig, "k-sse-3", "dev-3").await?;
    let SseSignal::Hint(Hint::News(mark)) = next_signal(&mut running).await? else {
        return Err("a new record must come as news".to_owned());
    };
    assert_eq!((mark.stream.as_str(), mark.seq), ("account/alice", 3));
    assert_eq!(mark.device_id, "dev-3");
    running.handle.abort();
    rig.served.stop().await
}

#[tokio::test]
async fn sse_session_without_any_byte_ends_idle_at_thirty_fake_seconds_after_connecting()
-> Result<(), String> {
    let rig = rig("sync_sse_idle").await?;
    let clock = FakeClock::new(CLIENT_START_MS);
    let http = http_on(&clock, None)?;
    let mut running = start_session(&http, &clock, target(&rig.base, &rig.token));
    assert_eq!(next_signal(&mut running).await?, SseSignal::Connected);
    let _progress = next_signal(&mut running).await?;
    let _woken = advance_clock(&clock, Duration::from_millis(29_999));
    settle().await;
    assert!(!running.handle.is_finished(), "still inside the 30 seconds");
    let _woken = advance_clock(&clock, Duration::from_millis(1));
    assert_eq!(end_of(running).await?, SseEnd::Idle);
    rig.served.stop().await
}

#[tokio::test]
async fn sse_session_heartbeat_bytes_move_the_thirty_second_window() -> Result<(), String> {
    let rig = rig("sync_sse_window").await?;
    let clock = FakeClock::new(CLIENT_START_MS);
    let http = http_on(&clock, None)?;
    let mut running = start_session(&http, &clock, target(&rig.base, &rig.token));
    assert_eq!(next_signal(&mut running).await?, SseSignal::Connected);
    let _progress = next_signal(&mut running).await?;
    let _woken = advance_clock(&clock, Duration::from_secs(20));
    walk(
        &rig.served.clock,
        Duration::from_secs(15),
        Duration::from_secs(1),
    )
    .await?;
    settle().await;
    let _woken = advance_clock(&clock, Duration::from_secs(29));
    settle().await;
    assert!(
        !running.handle.is_finished(),
        "the heartbeat at 20 s keeps the link until 50 s"
    );
    let _woken = advance_clock(&clock, Duration::from_secs(1));
    assert_eq!(end_of(running).await?, SseEnd::Idle);
    rig.served.stop().await
}

#[tokio::test]
async fn sse_session_with_a_wrong_credential_ends_credential_rejected() -> Result<(), String> {
    let rig = rig("sync_sse_401").await?;
    let clock = FakeClock::new(CLIENT_START_MS);
    let http = http_on(&clock, None)?;
    let running = start_session(&http, &clock, target(&rig.base, "not-the-key"));
    assert_eq!(end_of(running).await?, SseEnd::CredentialRejected);
    rig.served.stop().await
}

#[tokio::test]
async fn sse_session_stops_with_receiver_gone_when_nobody_listens_any_more() -> Result<(), String> {
    let rig = rig("sync_sse_gone").await?;
    let clock = FakeClock::new(CLIENT_START_MS);
    let http = http_on(&clock, None)?;
    let mut running = start_session(&http, &clock, target(&rig.base, &rig.token));
    assert_eq!(next_signal(&mut running).await?, SseSignal::Connected);
    let _progress = next_signal(&mut running).await?;
    let support::Running {
        handle, signals, ..
    } = running;
    drop(signals);
    append_one(&rig, "k-sse-gone", "dev-g").await?;
    let ended = test_harness::timeout_real(handle, Duration::from_secs(20))
        .await
        .map_err(|_| "no end".to_owned())?
        .map_err(|e| e.to_string())?;
    assert_eq!(ended, SseEnd::ReceiverGone);
    rig.served.stop().await
}
