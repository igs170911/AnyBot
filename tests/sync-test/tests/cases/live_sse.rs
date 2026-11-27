use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use anybot_store_client::LocalRecord;
use anybot_sync::SyncClient;
use anybot_sync::reconnect_status::LinkPhase;
use test_harness::netctl_modes::net_drop_events;
use test_harness::{FakeClock, sleep_real, test_dir, timeout_real};

use crate::common::{Counting, Rig, drive_until, rig};
use crate::scene::live::{device_login, drive_to_phase, write_steps};
use crate::scene::tap::Tap;
use crate::scene::{
    BOT_STREAM_ID, create_bot, local_bot_records, local_has, phone_sends, same_as_server,
};

const STEP: Duration = Duration::from_millis(250);
const CHAOS_STEP: Duration = Duration::from_secs(10);
const DISCONNECTS: u32 = 500;
const DROP_PER_MILLE: u16 = 300;

fn continued_from_41(after: &[LocalRecord], before: &[LocalRecord], queued: &[String]) {
    let total = before.len().saturating_add(queued.len());
    assert_eq!(after.get(..before.len()), Some(before));
    let seqs: Vec<i64> = after.iter().map(|record| record.seq).collect();
    let want: Vec<i64> = (1_i64..).take(total).collect();
    assert_eq!(seqs, want);
    let unique: HashSet<&String> = after.iter().map(|record| &record.record_id).collect();
    assert_eq!(unique.len(), total);
    let tail: Vec<&String> = after
        .iter()
        .skip(before.len())
        .map(|r| &r.record_id)
        .collect();
    let queued: Vec<&String> = queued.iter().collect();
    assert_eq!(tail, queued);
}

async fn runner_queues_while_stopped(
    rig: &mut Rig,
    a: &SyncClient,
    runner: &str,
) -> Result<(SyncClient, Vec<String>), String> {
    rig.served.stop_server().await?;
    drive_to_phase(a, &rig.clock(), STEP, LinkPhase::Retrying).await?;
    let b_dir = test_dir("sync_sse_2_b")?;
    let b = rig
        .client_as(runner, &rig.url(), &b_dir, Counting::arc(20))
        .await?;
    let queued = write_steps(&b, 5, "b").await?;
    rig.served.restart_server().await?;
    Ok((b, queued))
}

fn after_of(line: &str) -> Option<i64> {
    let (_head, rest) = line.split_once("after=")?;
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    digits.parse().ok()
}

fn asked_after_40(lines: &[String], before: &[LocalRecord]) -> Result<(), String> {
    let last = before.last().ok_or("A had nothing before the outage")?;
    let asks: Vec<&String> = lines
        .iter()
        .filter(|line| line.contains("stream=bot%2Fbot-1"))
        .collect();
    let first = asks
        .first()
        .ok_or("A never asked for the bot stream again")?;
    let resume = format!("after=40&device_at={}", last.record_id);
    assert!(
        first.contains(&resume),
        "first ask after the outage: {first}"
    );
    assert!(
        asks.iter()
            .all(|line| after_of(line).is_some_and(|after| after >= 40)),
        "asks after the outage: {asks:?}"
    );
    Ok(())
}

#[tokio::test]
async fn t_conn_sse_2() -> Result<(), String> {
    timeout_real(conn_sse_2(), Duration::from_secs(90)).await?
}

async fn conn_sse_2() -> Result<(), String> {
    let mut rig = rig("sync_sse_2").await?;
    create_bot(&rig).await?;
    phone_sends(&rig, 1, 40, "p").await?;
    let runner = device_login(&rig, "runner-b").await?;
    let clock = rig.clock();
    let tap = Tap::start(rig.served.origin_addr()).await?;
    let a_dir = test_dir("sync_sse_2_a")?;
    let a = rig.client_at(&tap.url(), &a_dir, Counting::arc(2)).await?;
    drive_until("A saw number 40", &clock, STEP, || local_has(&a, 40)).await?;
    let before = local_bot_records(&a).await?;
    let mark = tap.every_request().len();
    let (b, queued) = runner_queues_while_stopped(&mut rig, &a, &runner).await?;
    let total = before.len().saturating_add(queued.len());
    drive_until("A has every record", &clock, STEP, || local_has(&a, total)).await?;
    let after = local_bot_records(&a).await?;
    continued_from_41(&after, &before, &queued);
    let lines: Vec<String> = tap.every_request().into_iter().skip(mark).collect();
    asked_after_40(&lines, &before)?;
    assert!(same_as_server(
        &after,
        &rig.server_rows(BOT_STREAM_ID).await?
    ));
    drop(b);
    drop(a);
    rig.served.stop().await
}

fn connected_since(client: &SyncClient) -> Option<u64> {
    let link = client.status().link?;
    (link.phase == LinkPhase::Connected).then(|| link.since.millis())
}

async fn drive_to_new_connection(
    client: &SyncClient,
    clock: &Arc<FakeClock>,
    old: Option<u64>,
) -> Result<u64, String> {
    for _round in 0..400 {
        if let Some(since) = connected_since(client).filter(|since| Some(*since) != old) {
            return Ok(since);
        }
        sleep_real(Duration::from_millis(10)).await;
        let _woken = clock.advance(CHAOS_STEP);
    }
    Err(format!(
        "no new connection; last {:?}",
        client.status().link
    ))
}

async fn disconnect(rig: &Rig, client: &SyncClient, since: u64, seed: u64) -> Result<(), String> {
    for attempt in 0..50_u64 {
        net_drop_events(
            &rig.ctl,
            rig.proxy_port,
            DROP_PER_MILLE,
            seed.wrapping_add(attempt),
        )
        .await?;
        sleep_real(Duration::from_millis(20)).await;
        if connected_since(client) != Some(since) {
            return Ok(());
        }
    }
    Err("the link survived 50 cuts".to_owned())
}

fn next(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    *state >> 33
}

async fn churn(rig: &Rig, a: &SyncClient) -> Result<(u64, u64), String> {
    let clock = rig.clock();
    let mut state = 0x5eed_u64;
    let mut written = 0u64;
    let mut since = drive_to_new_connection(a, &clock, None).await?;
    for cut in 0..DISCONNECTS {
        let burst = next(&mut state).wrapping_rem(4);
        phone_sends(rig, written.saturating_add(1), burst, &format!("c{cut}")).await?;
        written = written.saturating_add(burst);
        for _hold in 0..next(&mut state).wrapping_rem(3) {
            sleep_real(Duration::from_millis(10)).await;
            let _woken = clock.advance(CHAOS_STEP);
        }
        disconnect(rig, a, since, u64::from(cut)).await?;
        since = drive_to_new_connection(a, &clock, Some(since)).await?;
    }
    Ok((since, written))
}

#[tokio::test]
async fn t_conn_sse_4() -> Result<(), String> {
    timeout_real(conn_sse_4(), Duration::from_secs(110)).await?
}

async fn conn_sse_4() -> Result<(), String> {
    let rig = rig("sync_sse_4").await?;
    create_bot(&rig).await?;
    let clock = rig.clock();
    net_drop_events(&rig.ctl, rig.proxy_port, DROP_PER_MILLE, 7).await?;
    let a = rig
        .client(&test_dir("sync_sse_4_a")?, Counting::arc(4))
        .await?;
    let (since, written) = churn(&rig, &a).await?;
    phone_sends(&rig, written.saturating_add(1), 3, "last").await?;
    disconnect(&rig, &a, since, 9_999).await?;
    let _since = drive_to_new_connection(&a, &clock, Some(since)).await?;
    let rows = rig.server_rows(BOT_STREAM_ID).await?;
    assert!(rows.len() > 500, "only {} records were written", rows.len());
    drive_until(
        "A's replica equals the server",
        &clock,
        CHAOS_STEP,
        || async { Ok(same_as_server(&local_bot_records(&a).await?, &rows)) },
    )
    .await?;
    drop(a);
    rig.served.stop().await
}
