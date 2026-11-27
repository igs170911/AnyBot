use serde_json::{Value, json};

use anybot_app_server as _;
use anybot_app_test as _;
use anybot_foundation as _;
use anybot_messages as _;
use anybot_server as _;
use anybot_server_test as _;
use anybot_store_server as _;
use axum as _;
use clap as _;
use rpassword as _;
use serde_json as _;
use sqlx as _;
use test_harness as _;
use tokio as _;
use tracing as _;

use std::time::Duration;

use anybot_app_test::serve::serve_start_with;
use anybot_domain::version_release::ReleaseTable;

mod pg_api_evidence_shared;
use pg_api_evidence_shared::{
    EVIDENCE_RECORD, FIRST_MESSAGE, TOOL_RECORD, act, bot_stream, conversation, create_bot,
    expect_accepted, fetch_record, login, record, record_count, result_row, send, stored,
};

const DAYS_PAST_THE_WINDOW: u64 = 31;

#[tokio::test]
async fn t_sync_append_6() -> Result<(), String> {
    let served = serve_start_with("evidence_rules", &ReleaseTable::empty()).await?;
    let token = login(&served).await?;
    create_bot(&served, &token).await?;
    let landed = send(
        &served,
        &token,
        "k-rules-1",
        &bot_stream(),
        &conversation("plain chat line"),
    )
    .await?;
    let message_seq = expect_accepted(&landed, 0)?;
    let tool_seq = expect_accepted(&landed, 2)?;
    let evidence_seq = expect_accepted(&landed, 3)?;
    let tool_before = fetch_record(&served, &token, tool_seq).await?.json;
    let evidence_before = fetch_record(&served, &token, evidence_seq).await?.json;
    let count_before = record_count(&served).await?;
    let direct: Vec<Value> = ["record_purge", "record_restore", "record_purge_permanent"]
        .iter()
        .enumerate()
        .map(|(index, kind)| {
            record(
                &format!("018f0000-0000-7000-8000-00000000010{index}"),
                kind,
                &json!({}),
                None,
            )
        })
        .collect();
    let forged = send(&served, &token, "k-rules-2", &bot_stream(), &direct).await?;
    for index in 0..3 {
        let row = result_row(&forged, index)?;
        assert_eq!(row.get("result"), Some(&json!("rejected")), "{row}");
        assert_eq!(
            row.get("code"),
            Some(&json!("append-kind-forbidden")),
            "{row}"
        );
        assert!(row.get("seq").is_none(), "a rejected record has no number");
    }
    assert_eq!(
        record_count(&served).await?,
        count_before,
        "nothing written"
    );
    let early = act(
        &served,
        &token,
        "k-rules-3",
        "purge_permanent",
        Some(message_seq),
    )
    .await?;
    assert_eq!(early.status, 400, "{}", early.json);
    assert_eq!(
        early.code().as_deref(),
        Some("record-action-target-invalid")
    );
    assert!(
        !stored(&served, FIRST_MESSAGE).await?.1,
        "message untouched"
    );
    let cleared = act(&served, &token, "k-rules-4", "clear", None).await?;
    assert_eq!(cleared.status, 200, "{}", cleared.json);
    let clear_seq = cleared
        .json
        .get("seq")
        .and_then(Value::as_i64)
        .ok_or("no clear seq")?;
    for (key, target) in [("k-rules-5", tool_seq), ("k-rules-6", evidence_seq)] {
        let refused = act(&served, &token, key, "purge_permanent", Some(target)).await?;
        assert_eq!(refused.status, 400, "{}", refused.json);
        assert_eq!(
            refused.code().as_deref(),
            Some("record-action-target-invalid")
        );
    }
    assert_eq!(
        fetch_record(&served, &token, tool_seq).await?.json,
        tool_before,
        "the action record content is still there"
    );
    assert_eq!(
        fetch_record(&served, &token, evidence_seq).await?.json,
        evidence_before,
        "the evidence content is still there"
    );
    assert!(!stored(&served, TOOL_RECORD).await?.1);
    assert!(!stored(&served, EVIDENCE_RECORD).await?.1);
    let _advanced = served
        .clock
        .advance(Duration::from_secs(DAYS_PAST_THE_WINDOW * 24 * 60 * 60));
    let late = act(&served, &token, "k-rules-7", "restore", Some(clear_seq)).await?;
    assert_eq!(late.status, 409, "{}", late.json);
    assert_eq!(late.code().as_deref(), Some("record-restore-expired"));
    served.stop().await
}
