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

use anybot_app_test::http::{RequestSpec, post};
use anybot_app_test::serve::serve_start_with;
use anybot_domain::version_release::ReleaseTable;

mod pg_api_streams_shared;
use pg_api_streams_shared::{
    ACCOUNT, admin_bearer, append, append_body, item, login_bearer, login_bob, result_code,
    result_seq, result_tag, streams_count, two_messages,
};

#[tokio::test]
async fn append_assigns_sequential_numbers_in_order() -> Result<(), String> {
    let served = serve_start_with("streams_seq", &ReleaseTable::empty()).await?;
    let base = served.base_url();
    let token = login_bearer(&served).await?;
    let answer = append(
        &base,
        &token,
        "k-seq-1",
        &append_body(ACCOUNT, &two_messages()),
    )
    .await?;
    assert_eq!(answer.status, 200, "append: {}", answer.json);
    assert_eq!(result_seq(&answer, 0)?, 1, "first seq: {}", answer.json);
    assert_eq!(result_seq(&answer, 1)?, 2, "second seq: {}", answer.json);
    served.stop().await
}

#[tokio::test]
async fn append_same_record_resends_as_duplicate() -> Result<(), String> {
    let served = serve_start_with("streams_dup", &ReleaseTable::empty()).await?;
    let base = served.base_url();
    let token = login_bearer(&served).await?;
    let one = vec![item("dev-a", "message", &json!({"text": "same"}))];
    let first = append(&base, &token, "k-dup-1", &append_body(ACCOUNT, &one)).await?;
    let seq = result_seq(&first, 0)?;
    let again = append(&base, &token, "k-dup-2", &append_body(ACCOUNT, &one)).await?;
    assert_eq!(
        result_tag(&again, 0)?,
        "duplicate",
        "replay: {}",
        again.json
    );
    assert_eq!(result_seq(&again, 0)?, seq, "same seq: {}", again.json);
    served.stop().await
}

#[tokio::test]
async fn append_used_device_id_with_other_content_is_rejected() -> Result<(), String> {
    let served = serve_start_with("streams_conflict", &ReleaseTable::empty()).await?;
    let base = served.base_url();
    let token = login_bearer(&served).await?;
    let first = vec![item("dev-a", "message", &json!({"text": "one"}))];
    let second = vec![item("dev-a", "message", &json!({"text": "two"}))];
    let _landed = append(&base, &token, "k-conf-1", &append_body(ACCOUNT, &first)).await?;
    let answer = append(&base, &token, "k-conf-2", &append_body(ACCOUNT, &second)).await?;
    assert_eq!(
        result_tag(&answer, 0)?,
        "rejected",
        "conflict: {}",
        answer.json
    );
    assert_eq!(
        result_code(&answer, 0)?,
        "record-id-conflict",
        "conflict code: {}",
        answer.json
    );
    served.stop().await
}

#[tokio::test]
async fn append_referee_and_record_kinds_are_forbidden_from_devices() -> Result<(), String> {
    let served = serve_start_with("streams_forbidden", &ReleaseTable::empty()).await?;
    let base = served.base_url();
    let token = login_bearer(&served).await?;
    let kinds = [
        "referee_cancel",
        "referee_stop",
        "referee_wake",
        "referee_card_answer",
        "referee_done_criteria",
        "record_delete",
        "record_purge",
        "record_restore",
        "record_purge_permanent",
        "bots_purge",
    ];
    let items: Vec<Value> = kinds
        .iter()
        .map(|k| item(&format!("dev-{k}"), k, &json!({})))
        .collect();
    let answer = append(&base, &token, "k-forbid", &append_body(ACCOUNT, &items)).await?;
    for index in 0..kinds.len() {
        assert_eq!(
            result_tag(&answer, index)?,
            "rejected",
            "kind {index}: {}",
            answer.json
        );
        assert_eq!(
            result_code(&answer, index)?,
            "append-kind-forbidden",
            "kind code {index}: {}",
            answer.json
        );
    }
    served.stop().await
}

#[tokio::test]
async fn append_replays_the_answer_for_the_same_key() -> Result<(), String> {
    let served = serve_start_with("streams_idem", &ReleaseTable::empty()).await?;
    let base = served.base_url();
    let token = login_bearer(&served).await?;
    let body = append_body(ACCOUNT, &two_messages());
    let first = append(&base, &token, "k-idem-1", &body).await?;
    let again = append(&base, &token, "k-idem-1", &body).await?;
    assert_eq!(again.status, 200, "replay: {}", again.json);
    assert_eq!(again.json, first.json, "same answer: {:?}", again.json);
    let count = streams_count(&served).await?;
    assert_eq!(count, 2, "no second insert: {count}");
    served.stop().await
}

#[tokio::test]
async fn append_batch_over_the_limit_is_refused() -> Result<(), String> {
    let served = serve_start_with("streams_limit", &ReleaseTable::empty()).await?;
    let base = served.base_url();
    let token = login_bearer(&served).await?;
    let items: Vec<Value> = (0..1001)
        .map(|n| item(&format!("dev-{n}"), "message", &json!({"text": n})))
        .collect();
    let answer = append(&base, &token, "k-limit", &append_body(ACCOUNT, &items)).await?;
    assert_eq!(answer.status, 400, "over 1000: {}", answer.json);
    served.stop().await
}

fn bot_create(bot_id: &str) -> Vec<Value> {
    vec![item(
        "dev-a",
        "bots_create",
        &json!({"bot_id": bot_id, "name": "helper"}),
    )]
}

#[tokio::test]
async fn append_bots_create_then_the_bot_stream_opens() -> Result<(), String> {
    let served = serve_start_with("streams_bot_create", &ReleaseTable::empty()).await?;
    let base = served.base_url();
    let token = login_bearer(&served).await?;
    let landed = append(
        &base,
        &token,
        "k-bot-1",
        &append_body(ACCOUNT, &bot_create("bot-1")),
    )
    .await?;
    assert_eq!(result_seq(&landed, 0)?, 1, "create: {}", landed.json);
    let missing = vec![item("dev-b", "message", &json!({"text": "hi"}))];
    let early = append(&base, &token, "k-bot-2", &append_body("bot-1", &missing)).await?;
    assert_eq!(
        result_seq(&early, 0)?,
        1,
        "bot stream opens: {}",
        early.json
    );
    served.stop().await
}

#[tokio::test]
async fn append_to_deleted_bot_is_rejected() -> Result<(), String> {
    let served = serve_start_with("streams_bot_deleted", &ReleaseTable::empty()).await?;
    let base = served.base_url();
    let token = login_bearer(&served).await?;
    let _landed = append(
        &base,
        &token,
        "k-del-1",
        &append_body(ACCOUNT, &bot_create("bot-2")),
    )
    .await?;
    let delete = vec![item("dev-b", "bots_delete", &json!({"bot_id": "bot-2"}))];
    let _gone = append(&base, &token, "k-del-2", &append_body(ACCOUNT, &delete)).await?;
    let message = vec![item("dev-c", "message", &json!({"text": "hi"}))];
    let answer = append(&base, &token, "k-del-3", &append_body("bot-2", &message)).await?;
    assert_eq!(
        result_tag(&answer, 0)?,
        "rejected",
        "deleted bot: {}",
        answer.json
    );
    assert_eq!(
        result_code(&answer, 0)?,
        "record-bot-deleted",
        "deleted code: {}",
        answer.json
    );
    served.stop().await
}

#[tokio::test]
async fn append_to_other_account_bot_is_refused() -> Result<(), String> {
    let served = serve_start_with("streams_bot_foreign", &ReleaseTable::empty()).await?;
    let base = served.base_url();
    let token = login_bearer(&served).await?;
    let _landed = append(
        &base,
        &token,
        "k-forg-1",
        &append_body(ACCOUNT, &bot_create("bot-3")),
    )
    .await?;
    let bob = login_bob(&served).await?;
    let message = vec![item("dev-b", "message", &json!({"text": "hi"}))];
    let answer = append(&base, &bob, "k-forg-2", &append_body("bot-3", &message)).await?;
    assert_eq!(answer.status, 200, "body status: {}", answer.json);
    assert_eq!(
        result_tag(&answer, 0)?,
        "rejected",
        "foreign bot: {}",
        answer.json
    );
    assert_eq!(
        result_code(&answer, 0)?,
        "record-no-permission",
        "foreign code: {}",
        answer.json
    );
    served.stop().await
}

#[tokio::test]
async fn append_key_with_other_fingerprint_conflicts() -> Result<(), String> {
    let served = serve_start_with("streams_key_conflict", &ReleaseTable::empty()).await?;
    let base = served.base_url();
    let token = login_bearer(&served).await?;
    let first = append(
        &base,
        &token,
        "k-mix",
        &append_body(ACCOUNT, &bot_create("bot-9")),
    )
    .await?;
    assert_eq!(first.status, 200, "first: {}", first.json);
    let other = vec![item("dev-c", "message", &json!({"text": "third"}))];
    let answer = append(&base, &token, "k-mix", &append_body(ACCOUNT, &other)).await?;
    assert_eq!(answer.status, 409, "fingerprint conflict: {}", answer.json);
    assert_eq!(
        answer.code().unwrap_or_default(),
        "account-idempotency-conflict"
    );
    served.stop().await
}

#[tokio::test]
async fn append_with_a_disabled_account_token_is_revoked() -> Result<(), String> {
    let served = serve_start_with("streams_disabled", &ReleaseTable::empty()).await?;
    let base = served.base_url();
    anybot_app_test::serve::boot_pair(&served).await?;
    let admin = admin_bearer(&served).await?;
    let bob = login_bob(&served).await?;
    let body = json!({"ok": true}).to_string();
    let spec = RequestSpec::new(&body)
        .with_key("k-disable")
        .with_bearer(&admin);
    let flipped = post(&base, "/v1/admin/accounts/bob/disable", &spec).await?;
    assert_eq!(flipped.status, 200, "disable: {}", flipped.json);
    let message = vec![item("dev-a", "message", &json!({"text": "hi"}))];
    let answer = append(&base, &bob, "k-disabled", &append_body(ACCOUNT, &message)).await?;
    assert_eq!(answer.status, 401, "revoked session: {}", answer.json);
    assert_eq!(
        answer.code().unwrap_or_default(),
        "account-login-credentials",
        "revoke code: {}",
        answer.json
    );
    served.stop().await
}
