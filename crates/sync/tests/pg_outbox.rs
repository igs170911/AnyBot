mod common;

use std::time::Duration;

use anybot_store_client::OutboxState;
use anybot_sync::{NewRecord, StreamTarget, SyncClient};
use test_harness::netctl::{net_down, net_drop_response, net_restore};

use common::{ACCOUNT_STREAM_ID, Counting, Rig, drive_until, message, rig, wait_until};

const STEP: Duration = Duration::from_millis(250);

fn chat(text: &str) -> NewRecord {
    NewRecord {
        stream: StreamTarget::Account,
        kind: "message".to_owned(),
        content: message(text),
        task_id: None,
    }
}

async fn add(client: &SyncClient, record: NewRecord) -> Result<String, String> {
    client.add_record(record).await.map_err(|e| e.to_string())
}

async fn outbox_len(client: &SyncClient) -> Result<usize, String> {
    Ok(client.outbox().await.map_err(|e| e.to_string())?.len())
}

async fn drained(rig: &Rig, client: &SyncClient, what: &str) -> Result<(), String> {
    drive_until(what, &rig.clock(), STEP, || async {
        Ok(outbox_len(client).await? == 0)
    })
    .await
}

async fn texts_on_server(rig: &Rig, stream_id: &str) -> Result<Vec<String>, String> {
    let rows = rig.server_rows(stream_id).await?;
    Ok(rows
        .iter()
        .filter_map(|row| {
            row.content
                .get("text")
                .and_then(|t| t.as_str())
                .map(str::to_owned)
        })
        .collect())
}

#[tokio::test]
async fn t_account_n5_3() -> Result<(), String> {
    let rig = rig("sync_account_n5_3").await?;
    let dir = test_harness::test_dir("sync_account_n5_3_client")?;
    net_drop_response(&rig.ctl, rig.proxy_port, true).await?;
    let client = rig.client(&dir, Counting::arc(0)).await?;
    let record_id = add(&client, chat("jia")).await?;
    wait_until("record reached the server", || async {
        Ok(rig.server_rows(ACCOUNT_STREAM_ID).await?.len() == 1)
    })
    .await?;
    assert!(
        !client
            .can_cancel(&record_id)
            .await
            .map_err(|e| e.to_string())?
    );
    let cancelled = client
        .cancel_unsent(&record_id)
        .await
        .map_err(|e| e.to_string())?;
    assert!(cancelled.is_none());
    assert_eq!(outbox_len(&client).await?, 1);
    net_restore(&rig.ctl, rig.proxy_port).await?;
    drained(&rig, &client, "outbox drained after the network came back").await?;
    let rows = rig.server_rows(ACCOUNT_STREAM_ID).await?;
    assert_eq!(rows.len(), 1);
    assert_eq!(
        rows.first().map(|row| row.device_id.clone()),
        Some(record_id)
    );
    drop(client);
    rig.served.stop().await
}

#[tokio::test]
async fn outbox_survives_reopening_and_keeps_the_order() -> Result<(), String> {
    let rig = rig("sync_outbox_reopen").await?;
    let dir = test_harness::test_dir("sync_outbox_reopen_client")?;
    net_down(&rig.ctl, rig.proxy_port).await?;
    let first = rig.client(&dir, Counting::arc(0)).await?;
    for text in ["1", "2", "3"] {
        add(&first, chat(text)).await?;
    }
    drop(first);
    let second = rig.client(&dir, Counting::arc(0)).await?;
    let waiting: Vec<String> = second
        .outbox()
        .await
        .map_err(|e| e.to_string())?
        .iter()
        .filter_map(|row| {
            row.content
                .get("text")
                .and_then(|t| t.as_str())
                .map(str::to_owned)
        })
        .collect();
    assert_eq!(waiting, ["1", "2", "3"]);
    net_restore(&rig.ctl, rig.proxy_port).await?;
    drained(&rig, &second, "outbox drained").await?;
    assert_eq!(
        texts_on_server(&rig, ACCOUNT_STREAM_ID).await?,
        ["1", "2", "3"]
    );
    drop(second);
    rig.served.stop().await
}

#[tokio::test]
async fn account_stream_goes_first_and_a_bot_stream_waits_for_its_creation() -> Result<(), String> {
    let rig = rig("sync_outbox_bot_gate").await?;
    let dir = test_harness::test_dir("sync_outbox_bot_gate_client")?;
    net_down(&rig.ctl, rig.proxy_port).await?;
    let client = rig.client(&dir, Counting::arc(0)).await?;
    let bot_chat = NewRecord {
        stream: StreamTarget::Bot("bot-1".to_owned()),
        kind: "message".to_owned(),
        content: message("hello bot"),
        task_id: None,
    };
    add(&client, bot_chat).await?;
    let create = NewRecord {
        stream: StreamTarget::Account,
        kind: "bots_create".to_owned(),
        content: serde_json::json!({"bot_id": "bot-1", "name": "Bee"}),
        task_id: None,
    };
    add(&client, create).await?;
    net_restore(&rig.ctl, rig.proxy_port).await?;
    drained(&rig, &client, "both streams delivered").await?;
    let rejected = client.status().rejected;
    assert_eq!(rejected, 0);
    assert_eq!(texts_on_server(&rig, "bot/bot-1").await?, ["hello bot"]);
    assert_eq!(rig.server_rows(ACCOUNT_STREAM_ID).await?.len(), 1);
    drop(client);
    rig.served.stop().await
}

#[tokio::test]
async fn refused_record_keeps_its_code_later_ones_go_and_it_expires_after_seven_days()
-> Result<(), String> {
    let rig = rig("sync_outbox_refused").await?;
    let dir = test_harness::test_dir("sync_outbox_refused_client")?;
    let client = rig.client(&dir, Counting::arc(0)).await?;
    let forbidden = NewRecord {
        stream: StreamTarget::Account,
        kind: "referee_stop".to_owned(),
        content: serde_json::json!({}),
        task_id: None,
    };
    let refused_id = add(&client, forbidden).await?;
    add(&client, chat("after")).await?;
    wait_until("the later record was delivered", || async {
        Ok(texts_on_server(&rig, ACCOUNT_STREAM_ID).await? == ["after"])
    })
    .await?;
    let rows = client.outbox().await.map_err(|e| e.to_string())?;
    let refused = rows.first().ok_or("refused row missing")?;
    assert_eq!(refused.record_id, refused_id);
    assert_eq!(refused.state, OutboxState::Rejected);
    assert_eq!(
        refused.reject_code.as_deref(),
        Some("append-kind-forbidden")
    );
    let _woken = rig.clock().advance(Duration::from_hours(169));
    wait_until("the refused record expired", || async {
        Ok(outbox_len(&client).await? == 0)
    })
    .await?;
    drop(client);
    rig.served.stop().await
}

#[tokio::test]
async fn waiting_more_than_five_seconds_offline_marks_the_record_sent_offline() -> Result<(), String>
{
    let rig = rig("sync_outbox_offline_flag").await?;
    let dir = test_harness::test_dir("sync_outbox_offline_flag_client")?;
    net_down(&rig.ctl, rig.proxy_port).await?;
    let client = rig.client(&dir, Counting::arc(0)).await?;
    let slow = add(&client, chat("slow")).await?;
    wait_until("first attempt failed", || async {
        Ok(client.status().connection != anybot_sync::ConnectionState::Connecting)
    })
    .await?;
    let _woken = rig.clock().advance(Duration::from_secs(6));
    net_restore(&rig.ctl, rig.proxy_port).await?;
    drained(&rig, &client, "slow record delivered").await?;
    let online = add(&client, chat("online")).await?;
    drained(&rig, &client, "online record delivered").await?;
    let rows = rig.server_rows(ACCOUNT_STREAM_ID).await?;
    let flag_of = |id: &str| {
        rows.iter()
            .find(|row| row.device_id == id)
            .map(|row| row.offline_sent)
    };
    assert_eq!(flag_of(&slow), Some(true));
    assert_eq!(flag_of(&online), Some(false));
    drop(client);
    rig.served.stop().await
}

#[tokio::test]
async fn waiting_less_than_five_seconds_offline_is_not_marked() -> Result<(), String> {
    let rig = rig("sync_outbox_offline_quick").await?;
    let dir = test_harness::test_dir("sync_outbox_offline_quick_client")?;
    net_down(&rig.ctl, rig.proxy_port).await?;
    let client = rig.client(&dir, Counting::arc(0)).await?;
    let quick = add(&client, chat("quick")).await?;
    wait_until("first attempt failed", || async {
        Ok(client.status().connection != anybot_sync::ConnectionState::Connecting)
    })
    .await?;
    net_restore(&rig.ctl, rig.proxy_port).await?;
    drained(&rig, &client, "quick record delivered").await?;
    let rows = rig.server_rows(ACCOUNT_STREAM_ID).await?;
    let flag = rows
        .iter()
        .find(|row| row.device_id == quick)
        .map(|row| row.offline_sent);
    assert_eq!(flag, Some(false));
    drop(client);
    rig.served.stop().await
}
