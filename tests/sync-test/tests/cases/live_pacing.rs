use std::sync::Arc;
use std::time::Duration;

use anybot_foundation::{Clock, Secret, TrustedCerts};
use anybot_sync::reconnect_status::{LinkPhase, LinkReason, LinkStatus};
use anybot_sync::{SyncClient, SyncConfig};
use test_harness::netctl::{
    net_attach, net_cut, net_hang_connect, net_restore, net_stall, net_start,
};
use test_harness::origin::start_origin;
use test_harness::{FakeClock, sleep_real, test_dir, timeout_real};

use crate::common::{CLIENT_VERSION, Counting, Rig, rig};
use crate::scene::live::{PhaseLog, drive_to_phase};

const START_MS: u64 = 40_000_000;
const UNREACHABLE: &str = "https://203.0.113.5:8443";
const FINE_STEP: Duration = Duration::from_millis(100);
const BASES: [u64; 7] = [1_000, 2_000, 4_000, 8_000, 16_000, 30_000, 30_000];

async fn hanging_proxy() -> Result<String, String> {
    let origin = timeout_real(start_origin(), Duration::from_secs(20))
        .await
        .map_err(|_| "origin start".to_owned())??;
    let (ctl, port) = net_start().await?;
    net_attach(&ctl, port, origin).await?;
    net_hang_connect(&ctl, port).await?;
    Ok(format!("http://127.0.0.1:{port}"))
}

async fn unreachable_client(clock: &Arc<FakeClock>, proxy: String) -> Result<SyncClient, String> {
    let shared: Arc<dyn Clock> = Arc::<FakeClock>::clone(clock);
    let config = SyncConfig {
        data_dir: test_dir("sync_n16_1_a")?,
        server_url: UNREACHABLE.to_owned(),
        credential: Secret::from_untrusted("any".to_owned()),
        client_version: CLIENT_VERSION.to_owned(),
        trusted: TrustedCerts::empty(),
        proxy: Some(proxy),
        clock: shared,
        random: Counting::arc(16),
    };
    SyncClient::start(config).await.map_err(|e| e.to_string())
}

fn retries(rows: &[LinkStatus]) -> usize {
    rows.iter()
        .filter(|link| link.phase == LinkPhase::Retrying)
        .count()
}

struct Attempt {
    started: u64,
    failed: LinkStatus,
}

fn attempts(rows: &[LinkStatus]) -> Vec<Attempt> {
    rows.iter()
        .zip(rows.iter().skip(1))
        .filter(|(first, second)| {
            first.phase == LinkPhase::Connecting && second.phase == LinkPhase::Retrying
        })
        .map(|(first, second)| Attempt {
            started: first.since.millis(),
            failed: *second,
        })
        .collect()
}

#[tokio::test]
async fn t_server_n16_1() -> Result<(), String> {
    timeout_real(server_n16_1(), Duration::from_secs(100)).await?
}

async fn server_n16_1() -> Result<(), String> {
    let clock = FakeClock::new(START_MS);
    let client = unreachable_client(&clock, hanging_proxy().await?).await?;
    let log = PhaseLog::start(&client);
    for _round in 0..3_000 {
        if retries(&log.rows()) >= 8 {
            break;
        }
        sleep_real(Duration::from_millis(10)).await;
        let _woken = clock.advance(FINE_STEP);
    }
    let tried = attempts(&log.rows());
    assert!(tried.len() >= 8, "only {} attempts", tried.len());
    for (index, attempt) in tried.iter().enumerate() {
        let took = attempt
            .failed
            .since
            .millis()
            .saturating_sub(attempt.started);
        assert!(
            (5_000..=5_100).contains(&took),
            "attempt {index} took {took} ms"
        );
        assert_eq!(attempt.failed.reason, Some(LinkReason::ConnectTimeout));
    }
    for (index, (pair, base)) in tried.windows(2).zip(BASES).enumerate() {
        let [before, after] = pair else {
            return Err("window of two expected".to_owned());
        };
        let failed_at = before.failed.since.millis();
        let planned = before
            .failed
            .retry_at
            .ok_or("a retry time")?
            .millis()
            .saturating_sub(failed_at);
        assert!(
            (base.saturating_add(500)..=base.saturating_add(2_000)).contains(&planned),
            "wait {index} planned {planned} ms, base {base}"
        );
        let gap = after.started.saturating_sub(failed_at);
        assert!(
            (planned..=planned.saturating_add(100)).contains(&gap),
            "wait {index} was {gap} ms, planned {planned}"
        );
    }
    drop(client);
    Ok(())
}

fn current(client: &SyncClient) -> Result<LinkStatus, String> {
    client.status().link.ok_or("no link status".to_owned())
}

async fn walk_real(clock: &Arc<FakeClock>, total: Duration, step: Duration) {
    let mut gone = Duration::ZERO;
    while gone < total {
        sleep_real(Duration::from_millis(10)).await;
        let _woken = clock.advance(step);
        gone = gone.saturating_add(step);
    }
}

async fn drop_after(rig: &Rig, client: &SyncClient, held: Duration) -> Result<LinkStatus, String> {
    let clock = rig.clock();
    drive_to_phase(client, &clock, FINE_STEP, LinkPhase::Connected).await?;
    walk_real(&clock, held, Duration::from_secs(1)).await;
    net_cut(&rig.ctl, rig.proxy_port).await?;
    drive_to_phase(client, &clock, Duration::ZERO, LinkPhase::Retrying).await?;
    let dropped = current(client)?;
    net_restore(&rig.ctl, rig.proxy_port).await?;
    Ok(dropped)
}

fn planned_wait(link: &LinkStatus) -> Result<u64, String> {
    let at = link.retry_at.ok_or("a retry time")?.millis();
    Ok(at.saturating_sub(link.since.millis()))
}

#[tokio::test]
async fn t_server_n16_3() -> Result<(), String> {
    timeout_real(server_n16_3(), Duration::from_secs(100)).await?
}

async fn server_n16_3() -> Result<(), String> {
    let rig = rig("sync_n16_3").await?;
    let clock = rig.clock();
    let client = rig
        .client(&test_dir("sync_n16_3_a")?, Counting::arc(30))
        .await?;
    let mut short_waits = Vec::new();
    for _short in 0..3 {
        let dropped = drop_after(&rig, &client, Duration::from_secs(3)).await?;
        short_waits.push(planned_wait(&dropped)?);
    }
    assert!(
        short_waits.last().is_some_and(|wait| *wait >= 4_500),
        "short links kept climbing: {short_waits:?}"
    );
    let dropped = drop_after(&rig, &client, Duration::from_secs(31)).await?;
    let wait = planned_wait(&dropped)?;
    assert!(
        (1_500..=3_000).contains(&wait),
        "after 31 s the wait was {wait} ms"
    );
    let retry_at = dropped.retry_at.ok_or("a retry time")?.millis();
    drive_to_phase(&client, &clock, FINE_STEP, LinkPhase::Connected).await?;
    let back = current(&client)?.since.millis();
    assert!(
        (retry_at..=retry_at.saturating_add(100)).contains(&back),
        "reconnected at {back}, planned {retry_at}"
    );
    drop(client);
    rig.served.stop().await
}

fn still_trying(client: &SyncClient) -> Result<(), String> {
    let waiting = current(client)?.phase;
    assert!(
        matches!(waiting, LinkPhase::Connecting | LinkPhase::Slow),
        "phase {waiting:?}"
    );
    Ok(())
}

fn gave_up_idle_at_30(client: &SyncClient, started: u64) -> Result<(), String> {
    let gave_up = current(client)?;
    assert_eq!(gave_up.phase, LinkPhase::Retrying);
    assert_eq!(gave_up.reason, Some(LinkReason::Idle));
    let idle_for = gave_up.since.millis().saturating_sub(started);
    assert!(
        (30_000..=30_100).contains(&idle_for),
        "gave up after {idle_for} ms"
    );
    Ok(())
}

#[tokio::test]
async fn full_client_behind_a_proxy_that_swallows_everything_gives_up_at_30_seconds_and_tries_again()
-> Result<(), String> {
    timeout_real(swallowed(), Duration::from_secs(100)).await?
}

async fn swallowed() -> Result<(), String> {
    let rig = rig("sync_link_swallow").await?;
    let clock = rig.clock();
    net_stall(&rig.ctl, rig.proxy_port).await?;
    let started = clock.now_ms();
    let client = rig
        .client(&test_dir("sync_link_swallow_a")?, Counting::arc(31))
        .await?;
    walk_real(&clock, Duration::from_millis(29_900), FINE_STEP).await;
    sleep_real(Duration::from_millis(100)).await;
    still_trying(&client)?;
    walk_real(&clock, Duration::from_millis(1_100), FINE_STEP).await;
    sleep_real(Duration::from_millis(100)).await;
    gave_up_idle_at_30(&client, started)?;
    let log = PhaseLog::start(&client);
    walk_real(&clock, Duration::from_secs(3), FINE_STEP).await;
    let again = log
        .rows()
        .iter()
        .any(|link| link.phase == LinkPhase::Connecting);
    assert!(again, "a new connection was tried: {:?}", log.rows());
    drop(client);
    rig.served.stop().await
}
