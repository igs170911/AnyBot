use axum::http::StatusCode;

use anybot_protocol::{RecordItem, StreamSelector, selector_key};
use anybot_store_server::{Pool, bots, streams};

use crate::api_streams_util::ResolvedStream;
use crate::req_write::{CallerDenied, denied_detail, internal_detail, internal_sqlx};

pub async fn project_bot_record(
    pool: &Pool,
    resolved: &ResolvedStream,
    item: &RecordItem,
) -> Result<(), CallerDenied> {
    if !item.kind.starts_with("bots_") {
        return Ok(());
    }
    if resolved.bot.is_some() {
        return Ok(());
    }
    let bot_id = content_bot_id(&item.content)?;
    let now = item.created_at_ms;
    match item.kind.as_str() {
        "bots_create" => create_bot(pool, resolved, &bot_id, &item.content, now).await,
        "bots_delete" => bots::set_bot_deleted(pool, &bot_id, true)
            .await
            .map_err(internal_sqlx),
        "bots_restore" => bots::set_bot_deleted(pool, &bot_id, false)
            .await
            .map_err(internal_sqlx),
        other => Err(denied_detail(
            StatusCode::BAD_REQUEST,
            &format!("unknown bots kind: {other}"),
        )),
    }
}

async fn create_bot(
    pool: &Pool,
    resolved: &ResolvedStream,
    bot_id: &str,
    content: &serde_json::Value,
    now: i64,
) -> Result<(), CallerDenied> {
    let name = content_name(content)?;
    bots::ensure_bot(pool, bot_id, &resolved.owner, &name, now)
        .await
        .map_err(internal_sqlx)?;
    let key = selector_key(
        &StreamSelector::Bot {
            id: bot_id.to_owned(),
        },
        &resolved.owner,
    );
    streams::ensure_stream(pool, &key, "bot", &resolved.owner, Some(bot_id), now)
        .await
        .map_err(internal_sqlx)
}

fn content_bot_id(content: &serde_json::Value) -> Result<String, CallerDenied> {
    let id = content
        .get("bot_id")
        .and_then(|v| v.as_str())
        .ok_or(internal_detail("bots record needs bot_id"))?;
    Ok(id.to_owned())
}

fn content_name(content: &serde_json::Value) -> Result<String, CallerDenied> {
    let name = content
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or(internal_detail("bots_create needs a name"))?;
    Ok(name.to_owned())
}
