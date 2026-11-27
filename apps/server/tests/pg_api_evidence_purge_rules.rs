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

use anybot_app_test::serve::{Served, serve_start_with};
use anybot_domain::version_release::ReleaseTable;

mod pg_api_evidence_shared;
use pg_api_evidence_shared::{
    EVIDENCE_RECORD, FIRST_MESSAGE, TOOL_RECORD, act, bot_stream, conversation, create_bot,
    expect_accepted, fetch_record, login, record, record_count, result_row, send, stored,
};

const DAYS_PAST_THE_WINDOW: u64 = 31;

struct Seeded {
    message: i64,
    tool: i64,
    evidence: i64,
}

async fn seed(served: &Served, token: &str) -> Result<Seeded, String> {
    create_bot(served, token).await?;
    let items = conversation("plain chat line");
    let landed = send(served, token, "k-rules-1", &bot_stream(), &items).await?;
    Ok(Seeded {
        message: expect_accepted(&landed, 0)?,
        tool: expect_accepted(&landed, 2)?,
        evidence: expect_accepted(&landed, 3)?,
    })
}

async fn assert_direct_actions_refused(served: &Served, token: &str) -> Result<(), String> {
    let before = record_count(served).await?;
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
    let forged = send(served, token, "k-rules-2", &bot_stream(), &direct).await?;
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
    assert_eq!(record_count(served).await?, before, "nothing written");
    Ok(())
}

async fn assert_purge_refused(
    served: &Served,
    token: &str,
    key: &str,
    target: i64,
) -> Result<(), String> {
    let refused = act(served, token, key, "purge_permanent", Some(target)).await?;
    assert_eq!(refused.status, 400, "{}", refused.json);
    assert_eq!(
        refused.code().as_deref(),
        Some("record-action-target-invalid")
    );
    Ok(())
}

#[tokio::test]
async fn t_sync_append_6() -> Result<(), String> {
    let served = serve_start_with("evidence_rules", &ReleaseTable::empty()).await?;
    let token = login(&served).await?;
    let seeded = seed(&served, &token).await?;
    let tool_before = fetch_record(&served, &token, seeded.tool).await?.json;
    let evidence_before = fetch_record(&served, &token, seeded.evidence).await?.json;
    assert_direct_actions_refused(&served, &token).await?;
    assert_purge_refused(&served, &token, "k-rules-3", seeded.message).await?;
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
    assert_purge_refused(&served, &token, "k-rules-5", seeded.tool).await?;
    assert_purge_refused(&served, &token, "k-rules-6", seeded.evidence).await?;
    let tool_after = fetch_record(&served, &token, seeded.tool).await?.json;
    let evidence_after = fetch_record(&served, &token, seeded.evidence).await?.json;
    assert_eq!(
        tool_after, tool_before,
        "the action record content is still there"
    );
    assert_eq!(
        evidence_after, evidence_before,
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
