use crate::common::{Counting, Rig, rig};
use crate::scene::tap::Tap;
use crate::scene::{
    BOT_STREAM_ID, create_bot, local_bot_records, phone_sends, same_as_server, wait_local,
};
use anybot_sync::SyncClient;

async fn open_client(rig: &Rig, tap: &Tap, dir: &std::path::Path) -> Result<SyncClient, String> {
    rig.client_at(&tap.url(), dir, Counting::arc(0)).await
}

fn after_requests(tap: &Tap, stream: &str, from: usize) -> Vec<String> {
    let wanted = format!("stream={}", stream.replace('/', "%2F"));
    tap.seen()
        .into_iter()
        .skip(from)
        .filter(|exchange| exchange.request.starts_with("GET /v1/streams/after"))
        .filter(|exchange| exchange.request.contains(&wanted))
        .map(|exchange| exchange.request)
        .collect()
}

async fn synced_then_closed(
    rig: &Rig,
    tap: &Tap,
    dir: &std::path::Path,
    count: usize,
) -> Result<(), String> {
    let first = open_client(rig, tap, dir).await?;
    wait_local(&first, count).await?;
    drop(first);
    Ok(())
}

#[tokio::test]
async fn t_sync_replica_1() -> Result<(), String> {
    let rig = rig("sync_replica_1").await?;
    let tap = Tap::start(rig.served.origin_addr()).await?;
    let dir = test_harness::test_dir("sync_replica_1_client")?;
    create_bot(&rig).await?;
    phone_sends(&rig, 10, 5, "before").await?;
    synced_then_closed(&rig, &tap, &dir, 5).await?;
    phone_sends(&rig, 100, 3, "while-closed").await?;
    let mark = tap.seen().len();
    let reopened = open_client(&rig, &tap, &dir).await?;
    wait_local(&reopened, 8).await?;
    let asked = after_requests(&tap, BOT_STREAM_ID, mark);
    assert!(!asked.is_empty());
    assert!(
        asked
            .iter()
            .all(|line| line.contains("after=5") && !line.contains("after=0")),
        "requests: {asked:?}"
    );
    let server = rig.server_rows(BOT_STREAM_ID).await?;
    assert!(same_as_server(
        &local_bot_records(&reopened).await?,
        &server
    ));
    drop(reopened);
    rig.served.stop().await
}

#[tokio::test]
async fn t_sync_after_1() -> Result<(), String> {
    let rig = rig("sync_after_1").await?;
    let tap = Tap::start(rig.served.origin_addr()).await?;
    let dir = test_harness::test_dir("sync_after_1_client")?;
    create_bot(&rig).await?;
    phone_sends(&rig, 10, 100, "first").await?;
    synced_then_closed(&rig, &tap, &dir, 100).await?;
    phone_sends(&rig, 1000, 2400, "rest").await?;
    let mark = tap.seen().len();
    let reopened = open_client(&rig, &tap, &dir).await?;
    wait_local(&reopened, 2500).await?;
    let asked = after_requests(&tap, BOT_STREAM_ID, mark);
    let afters: Vec<&str> = asked
        .iter()
        .filter_map(|line| line.split("after=").nth(1))
        .filter_map(|rest| rest.split(['&', ' ']).next())
        .collect();
    assert_eq!(afters, ["100", "1100", "2100"]);
    let local = local_bot_records(&reopened).await?;
    let server = rig.server_rows(BOT_STREAM_ID).await?;
    assert!(same_as_server(&local, &server));
    assert_eq!(local.first().map(|rec| rec.seq), Some(1));
    drop(reopened);
    rig.served.stop().await
}
