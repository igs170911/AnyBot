use std::future::Future;

use anybot_sync::SyncClient;

use crate::common::{Counting, Rig, drive_until, message, rig};
use crate::scene::tap::Tap;
use crate::scene::{
    BOT_STREAM_ID, bot, contiguous, create_bot, local_bot_records, local_has, phone_sends,
    same_as_server,
};
use anybot_sync::{NewRecord, StreamTarget};

const BACKUP: &str = "hourly_backup";
const DATABASE: &str = "anybot_test";

async fn back_up(rig: &Rig) -> Result<(), String> {
    rig.served.db.pg.snapshot_database(DATABASE, BACKUP).await
}

async fn restore_server(rig: &mut Rig) -> Result<(), String> {
    rig.served.stop_server().await?;
    rig.served.db.pg.restore_database(DATABASE, BACKUP).await?;
    rig.served.restart_server().await
}

const STEP: std::time::Duration = std::time::Duration::from_millis(250);

async fn wait_until<F, Fut>(rig: &Rig, what: &str, check: F) -> Result<(), String>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<bool, String>>,
{
    drive_until(what, &rig.clock(), STEP, check).await
}

async fn wait_local(rig: &Rig, client: &SyncClient, count: usize) -> Result<(), String> {
    wait_until(rig, "the replica has the records", || async {
        local_has(client, count).await
    })
    .await
}

async fn open(rig: &Rig, tap: &Tap, dir: &std::path::Path) -> Result<SyncClient, String> {
    rig.client_at(&tap.url(), dir, Counting::arc(0)).await
}

async fn send_five(client: &SyncClient) -> Result<Vec<String>, String> {
    let mut ids = Vec::new();
    for number in 0..5 {
        let record = NewRecord {
            stream: StreamTarget::Bot(crate::scene::BOT.to_owned()),
            kind: "message".to_owned(),
            content: message(&format!("a{number}")),
            task_id: None,
        };
        ids.push(client.add_record(record).await.map_err(|e| e.to_string())?);
    }
    Ok(ids)
}

async fn server_has(rig: &Rig, count: usize) -> Result<bool, String> {
    Ok(rig.server_rows(BOT_STREAM_ID).await?.len() == count)
}

async fn backed_up_with_five_sent(
    rig: &Rig,
    tap: &Tap,
    dir: &std::path::Path,
) -> Result<(SyncClient, Vec<String>), String> {
    create_bot(rig).await?;
    phone_sends(rig, 10, 2, "backed-up").await?;
    back_up(rig).await?;
    let client = open(rig, tap, dir).await?;
    wait_local(rig, &client, 2).await?;
    let ids = send_five(&client).await?;
    wait_until(rig, "the five reached the server", || async {
        server_has(rig, 7).await
    })
    .await?;
    wait_local(rig, &client, 7).await?;
    Ok((client, ids))
}

fn ids_from(rows: &[crate::common::ServerRow], first_seq: i64) -> Vec<String> {
    rows.iter()
        .filter(|row| row.seq >= first_seq)
        .map(|row| row.device_id.clone())
        .collect()
}

async fn assert_repaired(
    rig: &Rig,
    client: &SyncClient,
    total: usize,
) -> Result<Vec<crate::common::ServerRow>, String> {
    wait_until(rig, "server and replica agree", || async {
        let rows = rig.server_rows(BOT_STREAM_ID).await?;
        let local = local_bot_records(client).await?;
        Ok(rows.len() == total && same_as_server(&local, &rows))
    })
    .await?;
    let rows = rig.server_rows(BOT_STREAM_ID).await?;
    assert!(contiguous(&rows));
    let mut seen: Vec<&str> = rows.iter().map(|row| row.device_id.as_str()).collect();
    seen.sort_unstable();
    seen.dedup();
    assert_eq!(seen.len(), rows.len());
    Ok(rows)
}

#[tokio::test]
async fn t_sync_replica_3() -> Result<(), String> {
    let mut rig = rig("sync_replica_3").await?;
    let tap = Tap::start(rig.served.origin_addr()).await?;
    let dir = test_harness::test_dir("sync_replica_3_client")?;
    let (client, ids) = backed_up_with_five_sent(&rig, &tap, &dir).await?;
    restore_server(&mut rig).await?;
    assert_eq!(rig.server_rows(BOT_STREAM_ID).await?.len(), 2);
    client.pull_now(&bot());
    let rows = assert_repaired(&rig, &client, 7).await?;
    assert_eq!(ids_from(&rows, 3), ids);
    let notice = client.status().resyncs;
    assert_eq!(
        notice.last().map(|n| (n.stream.clone(), n.resent)),
        Some((BOT_STREAM_ID.to_owned(), 5))
    );
    drop(client);
    rig.served.stop().await
}

#[tokio::test]
async fn t_sync_after_3() -> Result<(), String> {
    let mut rig = rig("sync_after_3").await?;
    let tap = Tap::start(rig.served.origin_addr()).await?;
    let dir = test_harness::test_dir("sync_after_3_client")?;
    let (client, _ids) = backed_up_with_five_sent(&rig, &tap, &dir).await?;
    restore_server(&mut rig).await?;
    let mark = tap.seen().len();
    client.pull_now(&bot());
    assert_repaired(&rig, &client, 7).await?;
    let seen: Vec<_> = tap.seen().into_iter().skip(mark).collect();
    let ahead = seen.iter().position(|e| {
        e.request.starts_with("GET /v1/streams/after")
            && e.request.contains("after=7")
            && e.status == Some(409)
    });
    let resend = seen
        .iter()
        .position(|e| e.request.starts_with("POST /v1/streams/append"));
    assert!(ahead.is_some(), "no 409 for after=7 in {seen:?}");
    assert!(
        resend > ahead,
        "the missing records were sent before the 409: {seen:?}"
    );
    drop(client);
    rig.served.stop().await
}

#[tokio::test]
async fn t_sync_replica_5() -> Result<(), String> {
    let mut rig = rig("sync_replica_5").await?;
    let tap = Tap::start(rig.served.origin_addr()).await?;
    let dir = test_harness::test_dir("sync_replica_5_client")?;
    let (client, ids) = backed_up_with_five_sent(&rig, &tap, &dir).await?;
    drop(client);
    restore_server(&mut rig).await?;
    phone_sends(&rig, 200, 8, "after-restore").await?;
    let reopened = open(&rig, &tap, &dir).await?;
    let rows = assert_repaired(&rig, &reopened, 15).await?;
    assert_eq!(ids_from(&rows, 11), ids);
    let notice = reopened.status().resyncs;
    assert_eq!(notice.last().map(|n| n.resent), Some(5));
    drop(reopened);
    rig.served.stop().await
}
