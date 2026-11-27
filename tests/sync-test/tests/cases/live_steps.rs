use std::time::Duration;

use anybot_sync::reconnect_status::LinkPhase;
use std::sync::Arc;

use anybot_sync::SyncClient;
use test_harness::{FakeClock, test_dir, timeout_real};

use crate::common::{Counting, Rig, drive_until, rig};
use crate::scene::live::{
    Seen, drive_to_phase, numbered_from_one, phone_login, phone_reads, write_steps,
};
use crate::scene::{BOT_STREAM_ID, create_bot};

const STEP: Duration = Duration::from_millis(250);

async fn server_holds(rig: &Rig, ids: &[String]) -> Result<usize, String> {
    let rows = rig.server_rows(BOT_STREAM_ID).await?;
    Ok(rows
        .iter()
        .filter(|row| ids.contains(&row.device_id))
        .count())
}

fn shows_exactly(seen: &[Seen], ids: &[String]) -> bool {
    let shown: Vec<&String> = seen.iter().map(|row| &row.record_id).collect();
    shown.len() == ids.len() && shown.iter().zip(ids).all(|(left, right)| *left == right)
}

async fn phone_sees_all(rig: &Rig, bearer: &str, ids: &[String]) -> Result<bool, String> {
    let seen = phone_reads(rig, bearer).await?;
    Ok(shows_exactly(&seen, ids) && numbered_from_one(&seen))
}

async fn steps_while_stopped(rig: &mut Rig, client: &SyncClient) -> Result<Vec<String>, String> {
    let clock = rig.clock();
    drive_to_phase(client, &clock, STEP, LinkPhase::Connected).await?;
    rig.served.stop_server().await?;
    drive_to_phase(client, &clock, STEP, LinkPhase::Retrying).await?;
    write_steps(client, 3, "a").await
}

async fn nobody_sees_yet(rig: &Rig, phone: &str, ids: &[String]) -> Result<(), String> {
    assert!(
        phone_reads(rig, phone).await.is_err(),
        "P cannot reach the stopped server"
    );
    assert_eq!(server_holds(rig, ids).await?, 0);
    Ok(())
}

async fn nothing_shown_before_opening(
    rig: &Rig,
    phone: &str,
    ids: &[String],
) -> Result<(), String> {
    let before = phone_reads(rig, phone).await?;
    assert!(before.is_empty(), "P saw {before:?} before A opened");
    assert_eq!(server_holds(rig, ids).await?, 0);
    Ok(())
}

async fn outbox_empties(client: &SyncClient, clock: &Arc<FakeClock>) -> Result<(), String> {
    drive_until("A's outbox is empty", clock, STEP, || async {
        Ok(client.status().pending == 0)
    })
    .await
}

#[tokio::test]
async fn t_sync_offline_3() -> Result<(), String> {
    timeout_real(sync_offline_3(), Duration::from_secs(90)).await?
}

async fn sync_offline_3() -> Result<(), String> {
    let mut rig = rig("sync_offline_3").await?;
    create_bot(&rig).await?;
    let phone = phone_login(&rig).await?;
    let clock = rig.clock();
    let client = rig
        .client(&test_dir("sync_offline_3_a")?, Counting::arc(3))
        .await?;
    let ids = steps_while_stopped(&mut rig, &client).await?;
    let _woken = clock.advance(Duration::from_secs(20));
    nobody_sees_yet(&rig, &phone, &ids).await?;
    rig.served.restart_server().await?;
    drive_to_phase(&client, &clock, STEP, LinkPhase::Connected).await?;
    let back_at = client
        .status()
        .link
        .map(|link| link.since.millis())
        .ok_or("no link")?;
    drive_until("P sees the three steps", &clock, STEP, || {
        phone_sees_all(&rig, &phone, &ids)
    })
    .await?;
    let took = clock.now_ms().saturating_sub(back_at);
    assert!(
        took <= 10_000,
        "P saw the steps {took} ms after A came back"
    );
    outbox_empties(&client, &clock).await?;
    drop(client);
    rig.served.stop().await
}

#[tokio::test]
async fn t_sync_offline_4() -> Result<(), String> {
    timeout_real(sync_offline_4(), Duration::from_secs(90)).await?
}

async fn sync_offline_4() -> Result<(), String> {
    let mut rig = rig("sync_offline_4").await?;
    create_bot(&rig).await?;
    let phone = phone_login(&rig).await?;
    let clock = rig.clock();
    let dir = test_dir("sync_offline_4_a")?;
    let client = rig.client(&dir, Counting::arc(4)).await?;
    let ids = steps_while_stopped(&mut rig, &client).await?;
    client.close().await;
    rig.served.restart_server().await?;
    let _woken = clock.advance(Duration::from_secs(3_600));
    nothing_shown_before_opening(&rig, &phone, &ids).await?;
    let reopened = rig.client(&dir, Counting::arc(40)).await?;
    drive_until("P sees the three steps", &clock, STEP, || {
        phone_sees_all(&rig, &phone, &ids)
    })
    .await?;
    outbox_empties(&reopened, &clock).await?;
    drop(reopened);
    rig.served.stop().await
}
