mod rig;
mod support;

use std::sync::Arc;
use std::time::Duration;

use anybot_foundation::Clock;
use anybot_server_test::releases;
use anybot_sync::reconnect_pacing::ReconnectPacer;
use anybot_sync::reconnect_random::SeededRandom;
use anybot_sync::sse_session::{SseEnd, SseSignal};
use rig::{Rig, next_signal, rig, rig_with};
use support::{CLIENT_VERSION, end_of, http_on, settle, start_session, target, walk};
use test_harness::netctl::{net_attach, net_cut, net_restore, net_stall, net_start};
use test_harness::netctl_modes::net_throttle;
use test_harness::{FakeClock, NetCtl, advance_clock};

use anybot_app_test as _;
use anybot_domain as _;
use anybot_foundation as _;
use serde_json as _;
use tokio as _;

const CLIENT_START_MS: u64 = 9_000_000;
const DAY_MS: i64 = 86_400_000;
const BASE_MS: i64 = 1_700_000_000_000;

async fn proxied(rig: &Rig) -> Result<(Arc<NetCtl>, u16, String), String> {
    let (ctl, port) = net_start().await?;
    net_attach(&ctl, port, rig.served.origin_addr()).await?;
    Ok((ctl, port, format!("http://127.0.0.1:{port}")))
}

async fn close_proxy(ctl: &Arc<NetCtl>) -> Result<(), String> {
    ctl.cut_all().await?;
    ctl.drain_kills().await;
    Ok(())
}

#[tokio::test]
async fn sse_session_through_a_proxy_that_swallows_everything_ends_idle_at_30_fake_seconds()
-> Result<(), String> {
    let rig = rig("sync_link_stall").await?;
    let (ctl, port, url) = proxied(&rig).await?;
    net_stall(&ctl, port).await?;
    let clock = FakeClock::new(CLIENT_START_MS);
    let http = http_on(&clock, None)?;
    let running = start_session(&http, &clock, target(&url, &rig.token));
    settle().await;
    let _woken = advance_clock(&clock, Duration::from_millis(29_999));
    settle().await;
    assert!(!running.handle.is_finished(), "inside the 30 seconds");
    let _woken = advance_clock(&clock, Duration::from_millis(1));
    assert_eq!(end_of(running).await?, SseEnd::Idle);
    close_proxy(&ctl).await?;
    rig.served.stop().await
}

#[tokio::test]
async fn sse_session_on_a_slow_link_that_keeps_delivering_bytes_is_not_cut_by_the_30_second_rule()
-> Result<(), String> {
    let rig = rig("sync_link_slow").await?;
    let (ctl, port, url) = proxied(&rig).await?;
    let clock = Arc::clone(&rig.served.clock);
    net_throttle(&ctl, port, 50, &clock).await?;
    let http = http_on(&clock, None)?;
    let mut running = start_session(&http, &clock, target(&url, &rig.token));
    walk(&clock, Duration::from_secs(120), Duration::from_secs(1)).await?;
    assert!(
        !running.handle.is_finished(),
        "a slow link is not a dead link"
    );
    assert_eq!(next_signal(&mut running).await?, SseSignal::Connected);
    let SseSignal::Hint(_progress) = next_signal(&mut running).await? else {
        return Err("the progress hint must have arrived over the slow link".to_owned());
    };
    running.handle.abort();
    close_proxy(&ctl).await?;
    rig.served.stop().await
}

#[tokio::test]
async fn sse_session_with_a_version_the_server_has_retired_ends_version_too_old()
-> Result<(), String> {
    let table = releases(&[("0.2.0", BASE_MS)]);
    let rig = rig_with("sync_link_old", &table).await?;
    let days = u64::try_from(220 * DAY_MS).map_err(|e| e.to_string())?;
    let _advanced = rig.served.clock.advance(Duration::from_millis(days));
    let clock = FakeClock::new(CLIENT_START_MS);
    let http = http_on(&clock, None)?;
    let running = start_session(&http, &clock, target(&rig.base, &rig.token));
    assert_eq!(end_of(running).await?, SseEnd::VersionTooOld);
    assert_eq!(CLIENT_VERSION, "0.1.0");
    rig.served.stop().await
}

#[tokio::test]
async fn sse_session_after_a_server_restart_ends_and_a_new_one_starts_with_progress_again()
-> Result<(), String> {
    let mut rig = rig("sync_link_restart").await?;
    rig::append_one(&rig, "k-restart-1", "dev-r1").await?;
    let clock = FakeClock::new(CLIENT_START_MS);
    let http = http_on(&clock, None)?;
    let mut first = start_session(&http, &clock, target(&rig.base, &rig.token));
    assert_eq!(next_signal(&mut first).await?, SseSignal::Connected);
    let _progress = next_signal(&mut first).await?;
    rig.served.stop_server().await?;
    let ended = end_of(first).await?;
    assert!(
        matches!(ended, SseEnd::ClosedByServer | SseEnd::Dropped(_)),
        "ended: {ended:?}"
    );
    rig.served.restart_server().await?;
    let base = rig.served.base_url();
    let mut second = start_session(&http, &clock, target(&base, &rig.token));
    assert_eq!(next_signal(&mut second).await?, SseSignal::Connected);
    let SseSignal::Hint(anybot_sync::sse_hint::Hint::Progress { streams }) =
        next_signal(&mut second).await?
    else {
        return Err("a restarted server must open with the progress".to_owned());
    };
    assert!(
        streams.iter().any(|mark| mark.seq == 1),
        "progress: {streams:?}"
    );
    second.handle.abort();
    rig.served.stop().await
}

#[tokio::test]
async fn reconnect_pacing_after_a_link_that_held_31_seconds_starts_again_from_one_second()
-> Result<(), String> {
    let rig = rig("sync_link_stable").await?;
    let (ctl, port, url) = proxied(&rig).await?;
    let clock = Arc::clone(&rig.served.clock);
    let http = http_on(&clock, None)?;
    let shared: Arc<dyn Clock> = Arc::<FakeClock>::clone(&clock);
    let pacer = ReconnectPacer::new(shared, Arc::new(SeededRandom::new(5)));
    for held in [3u64, 3, 31] {
        let mut running = start_session(&http, &clock, target(&url, &rig.token));
        assert_eq!(next_signal(&mut running).await?, SseSignal::Connected);
        pacer.connected();
        walk(&clock, Duration::from_secs(held), Duration::from_secs(1)).await?;
        net_cut(&ctl, port).await?;
        let ended = end_of(running).await?;
        assert!(
            matches!(ended, SseEnd::ClosedByServer | SseEnd::Dropped(_)),
            "ended: {ended:?}"
        );
        let delay = u64::try_from(pacer.failed().map_err(|e| e.reason)?.as_millis())
            .map_err(|e| e.to_string())?;
        let want = match held {
            31 => 1_500..=3_000,
            _short if pacer.failures_in_row() == 1 => 1_500..=3_000,
            _later => 2_500..=4_000,
        };
        assert!(want.contains(&delay), "held {held} s, delay {delay} ms");
        net_restore(&ctl, port).await?;
        test_harness::sleep_real(Duration::from_millis(400)).await;
    }
    close_proxy(&ctl).await?;
    rig.served.stop().await
}
