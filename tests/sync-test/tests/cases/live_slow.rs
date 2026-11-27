use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use anybot_sync::SyncClient;
use anybot_sync::reconnect_status::LinkPhase;
use test_harness::netctl::net_cut;
use test_harness::netctl_modes::net_throttle;
use test_harness::{FakeClock, sleep_real, test_dir, timeout_real};

use crate::common::{Counting, Rig, ServerRow, rig};
use crate::scene::live::{PhaseLog, drive_to_phase};
use crate::scene::tap::Tap;
use crate::scene::{
    BOT_STREAM_ID, create_bot, local_bot_records, phone_sends_sized, same_as_server,
};

const RECORDS: u64 = 2_500;
const RECORD_TEXT: usize = 2_048;
const RATE: u32 = 2_048;
const STEP: Duration = Duration::from_millis(250);
const SLOW_STEP: Duration = Duration::from_secs(5);
const BEAT_MS: u64 = 15_000;
const STEP_MS: u64 = 5_000;
const FIVE_MB: u64 = 5_000_000;

fn sse_sessions(tap: &Tap, opened: impl Fn(u64) -> bool) -> usize {
    tap.seen()
        .iter()
        .filter(|exchange| exchange.request.contains("/v1/streams/notifications"))
        .filter(|exchange| exchange.at_ms.is_some_and(&opened))
        .count()
}

fn sse_sessions_since(tap: &Tap, from_ms: u64) -> usize {
    sse_sessions(tap, |at| at >= from_ms)
}

fn sse_sessions_after(tap: &Tap, after_ms: u64) -> usize {
    sse_sessions(tap, |at| at > after_ms)
}

fn beat_gaps(beats: &[u64], from_ms: u64, to_ms: u64) -> Vec<u64> {
    let inside: Vec<u64> = beats
        .iter()
        .copied()
        .filter(|at| *at >= from_ms && *at <= to_ms)
        .collect();
    inside
        .iter()
        .zip(inside.iter().skip(1))
        .map(|(before, after)| after.saturating_sub(*before))
        .collect()
}

#[tokio::test]
async fn t_account_n8_1() -> Result<(), String> {
    timeout_real(
        Box::pin(slow_catch_up("sync_n8_1")),
        Duration::from_secs(110),
    )
    .await?
}

#[tokio::test]
async fn t_conn_heartbeat_5() -> Result<(), String> {
    timeout_real(
        Box::pin(slow_catch_up("sync_hb_5")),
        Duration::from_secs(110),
    )
    .await?
}

async fn slow_catch_up(name: &str) -> Result<(), String> {
    let rig = rig(name).await?;
    create_bot(&rig).await?;
    let clock = rig.clock();
    let upstream = SocketAddr::from(([127, 0, 0, 1], rig.proxy_port));
    let tap = Tap::start_timed(upstream, Some(Arc::clone(&clock))).await?;
    let dir = test_dir(&format!("{name}_a"))?;
    let a = rig.client_at(&tap.url(), &dir, Counting::arc(8)).await?;
    drive_to_phase(&a, &clock, STEP, LinkPhase::Connected).await?;
    net_cut(&rig.ctl, rig.proxy_port).await?;
    drive_to_phase(&a, &clock, STEP, LinkPhase::Retrying).await?;
    let rows = five_megabytes_behind(&rig).await?;
    let slowed_at = clock.now_ms();
    net_throttle(&rig.ctl, rig.proxy_port, RATE, &clock).await?;
    let log = PhaseLog::start(&a);
    drive_to_phase(&a, &clock, SLOW_STEP, LinkPhase::Connected).await?;
    let back_at = clock.now_ms();
    catch_up(&a, &clock, &rows).await?;
    let done_at = clock.now_ms();
    never_dropped(&log, &tap, slowed_at, back_at);
    beats_kept_coming(&tap.beats(), back_at, done_at)?;
    drop(a);
    rig.served.stop().await
}

async fn five_megabytes_behind(rig: &Rig) -> Result<Vec<ServerRow>, String> {
    phone_sends_sized(rig, RECORDS, RECORD_TEXT, 50).await?;
    let rows = rig.server_rows(BOT_STREAM_ID).await?;
    let bytes: u64 = rows
        .iter()
        .map(|row| row.content.to_string().len() as u64)
        .sum();
    assert!(bytes >= FIVE_MB, "only {bytes} bytes to catch up");
    Ok(rows)
}

async fn catch_up(
    a: &SyncClient,
    clock: &Arc<FakeClock>,
    rows: &[ServerRow],
) -> Result<(), String> {
    for round in 0..2_000_u32 {
        if round.checked_rem(20) == Some(0) && same_as_server(&local_bot_records(a).await?, rows) {
            return Ok(());
        }
        sleep_real(Duration::from_millis(10)).await;
        let _woken = clock.advance(SLOW_STEP);
    }
    Err("A never caught up".to_owned())
}

fn never_dropped(log: &PhaseLog, tap: &Tap, slowed_at: u64, back_at: u64) {
    assert!(sse_sessions_since(tap, slowed_at) >= 1);
    assert_eq!(sse_sessions_after(tap, back_at), 0);
    let dropped: Vec<_> = log
        .rows()
        .into_iter()
        .filter(|link| link.since.millis() > back_at)
        .filter(|link| matches!(link.phase, LinkPhase::Retrying | LinkPhase::Connecting))
        .collect();
    assert!(dropped.is_empty(), "the link was dropped: {dropped:?}");
}

fn beats_kept_coming(beats: &[u64], from_ms: u64, to_ms: u64) -> Result<(), String> {
    let gaps = beat_gaps(beats, from_ms, to_ms);
    let took = to_ms.saturating_sub(from_ms);
    assert!(took >= 2_000_000, "the catch-up took only {took} ms");
    let expected = usize::try_from(took / BEAT_MS).map_err(|e| e.to_string())?;
    assert!(
        gaps.len().saturating_add(3) >= expected,
        "{} beats in {took} ms",
        gaps.len()
    );
    let bounds = BEAT_MS.saturating_sub(STEP_MS)..=BEAT_MS.saturating_add(STEP_MS);
    assert!(
        gaps.iter().all(|gap| bounds.contains(gap)),
        "beat gaps {gaps:?}"
    );
    Ok(())
}
