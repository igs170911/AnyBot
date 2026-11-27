use std::time::Duration;

use anybot_app_test::http::{RequestSpec, get};
use anybot_store_client::OutboxState;
use anybot_sync::SyncClient;
use serde_json::{Value, json};
use test_harness::netctl::{net_down, net_restore};

use crate::common::{ACCOUNT_STREAM_ID, Counting, Rig, drive_until, rig};

const STEP: Duration = Duration::from_millis(250);

async fn outbox_len(client: &SyncClient) -> Result<usize, String> {
    Ok(client.outbox().await.map_err(|e| e.to_string())?.len())
}

async fn drained(rig: &Rig, client: &SyncClient, what: &str) -> Result<(), String> {
    drive_until(what, &rig.clock(), STEP, || async {
        Ok(outbox_len(client).await? == 0)
    })
    .await
}

async fn phone_settings(rig: &Rig) -> Result<Vec<(String, String)>, String> {
    let spec = RequestSpec::new("").with_bearer(&rig.bearer);
    let answer = get(&rig.served.base_url(), "/v1/settings", &spec).await?;
    let list = answer
        .json
        .get("settings")
        .and_then(Value::as_array)
        .ok_or("no settings list")?;
    Ok(list
        .iter()
        .filter_map(|entry| {
            let field = entry.get("field")?.as_str()?.to_owned();
            let value = entry.get("value")?.as_str()?.to_owned();
            Some((field, value))
        })
        .collect())
}

async fn local_settings(client: &SyncClient) -> Result<Vec<(String, String)>, String> {
    let rows = client.settings().await.map_err(|e| e.to_string())?;
    Ok(rows.into_iter().map(|s| (s.field, s.value)).collect())
}

fn green() -> Vec<(String, String)> {
    vec![("accent_color".to_owned(), "green".to_owned())]
}

#[tokio::test]
async fn t_account_n5_4() -> Result<(), String> {
    let rig = rig("sync_account_n5_4").await?;
    let dir = test_harness::test_dir("sync_account_n5_4_client")?;
    net_down(&rig.ctl, rig.proxy_port).await?;
    let client = rig.client(&dir, Counting::arc(0)).await?;
    client
        .change_setting("accent_color", None, "green")
        .await
        .map_err(|e| e.to_string())?;
    assert_eq!(local_settings(&client).await?, green());
    assert!(rig.server_rows(ACCOUNT_STREAM_ID).await?.is_empty());
    let _woken = rig.clock().advance(Duration::from_secs(10));
    net_restore(&rig.ctl, rig.proxy_port).await?;
    drained(&rig, &client, "the setting reached the server").await?;
    assert_eq!(phone_settings(&rig).await?, green());
    drive_until(
        "the replica holds the setting",
        &rig.clock(),
        STEP,
        || async {
            let held = client.records(&anybot_sync::StreamTarget::Account).await;
            Ok(held.is_ok_and(|rows| rows.iter().any(|row| row.kind == "setting")))
        },
    )
    .await?;
    assert_eq!(local_settings(&client).await?, green());
    let rows = rig.server_rows(ACCOUNT_STREAM_ID).await?;
    let sent = rows.first().ok_or("no record on the server")?;
    assert_eq!(sent.kind, "setting");
    assert!(sent.offline_sent);
    drop(client);
    rig.served.stop().await
}

#[tokio::test]
async fn a_setting_the_server_refuses_is_marked_not_sent_and_leaves_the_merge() -> Result<(), String>
{
    let rig = rig("sync_setting_refused").await?;
    let dir = test_harness::test_dir("sync_setting_refused_client")?;
    let client = rig.client(&dir, Counting::arc(0)).await?;
    client
        .change_setting("theme", None, "dark")
        .await
        .map_err(|e| e.to_string())?;
    drive_until("the refusal came back", &rig.clock(), STEP, || async {
        let rows = client.outbox().await.map_err(|e| e.to_string())?;
        Ok(rows.iter().all(|row| row.state == OutboxState::Rejected) && !rows.is_empty())
    })
    .await?;
    assert!(rejected_is_hidden(&client).await?);
    assert!(phone_settings(&rig).await?.is_empty());
    drop(client);
    rig.served.stop().await
}

async fn rejected_is_hidden(client: &SyncClient) -> Result<bool, String> {
    let shown = local_settings(client).await?;
    Ok(!shown.iter().any(|(field, _)| field == "theme"))
}

#[tokio::test]
async fn the_clock_offset_in_a_sent_setting_is_the_one_measured_this_connection()
-> Result<(), String> {
    let rig = rig("sync_setting_offset").await?;
    let dir = test_harness::test_dir("sync_setting_offset_client")?;
    let client = rig.client(&dir, Counting::arc(0)).await?;
    client
        .change_setting("density", None, "compact")
        .await
        .map_err(|e| e.to_string())?;
    drained(&rig, &client, "the setting reached the server").await?;
    let rows = rig.server_rows(ACCOUNT_STREAM_ID).await?;
    let content = rows
        .first()
        .map(|row| row.content.clone())
        .ok_or("no row")?;
    assert_eq!(content.get("field"), Some(&json!("density")));
    assert!(content.get("offset_ms").is_some_and(Value::is_i64));
    drop(client);
    rig.served.stop().await
}
