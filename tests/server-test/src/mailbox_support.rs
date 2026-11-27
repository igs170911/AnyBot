use serde_json::json;
use sqlx::Row as _;

use anybot_app_test::http::{RequestSpec, ResponseSpec, post};
use anybot_app_test::serve::Served;

pub struct LetterState {
    pub state: String,
    pub claimed_by: Option<String>,
    pub delivered_at_ms: Option<i64>,
    pub reassigned_to: Option<String>,
    pub device_id: String,
}

pub async fn deliver(
    served: &Served,
    bearer: &str,
    bot: &str,
    computer: Option<&str>,
    key: &str,
) -> Result<ResponseSpec, String> {
    let body = json!({"bot": bot, "computer": computer});
    let spec = RequestSpec::new(&body.to_string())
        .with_key(key)
        .with_bearer(bearer);
    post(&served.base_url(), "/v1/mailbox/round", &spec).await
}

pub async fn claim(
    served: &Served,
    bearer: &str,
    letter_id: &str,
    key: &str,
) -> Result<ResponseSpec, String> {
    let body = json!({"letter_id": letter_id});
    let spec = RequestSpec::new(&body.to_string())
        .with_key(key)
        .with_bearer(bearer);
    post(&served.base_url(), "/v1/mailbox/claim", &spec).await
}

pub async fn letter_state(served: &Served, letter_id: &str) -> Result<LetterState, String> {
    let row = sqlx::query(
        "SELECT state, claimed_by, delivered_at_ms, reassigned_to, device_id FROM mailbox_letters WHERE letter_id = $1",
    )
    .bind(letter_id)
    .fetch_one(&served.pool)
    .await
    .map_err(|e| e.to_string())?;
    Ok(LetterState {
        state: row.try_get("state").map_err(|e| e.to_string())?,
        claimed_by: row.try_get("claimed_by").map_err(|e| e.to_string())?,
        delivered_at_ms: row.try_get("delivered_at_ms").map_err(|e| e.to_string())?,
        reassigned_to: row.try_get("reassigned_to").map_err(|e| e.to_string())?,
        device_id: row.try_get("device_id").map_err(|e| e.to_string())?,
    })
}

pub async fn reassign_lines(served: &Served, bot: &str) -> Result<Vec<String>, String> {
    let stream = format!("bot/{bot}");
    sqlx::query_scalar::<_, String>(
        "SELECT content FROM records WHERE stream_id = $1 AND kind = 'mailbox_reassigned' ORDER BY seq",
    )
    .bind(stream)
    .fetch_all(&served.pool)
    .await
    .map_err(|e| e.to_string())
}

pub async fn pending_count(served: &Served, device_id: &str) -> Result<i64, String> {
    sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM mailbox_letters WHERE device_id = $1 AND state = 'pending'",
    )
    .bind(device_id)
    .fetch_one(&served.pool)
    .await
    .map_err(|e| e.to_string())
}
