use axum::http::StatusCode;

use anybot_protocol::{RecordItem, StreamSelector, selector_key};
use anybot_store_server::streams_projection;

use crate::api_streams_util::ResolvedStream;
use crate::req_write::{CallerDenied, denied_detail, internal_detail, internal_sqlx};

type Tx<'a> = sqlx::Transaction<'a, sqlx::Postgres>;

pub async fn project_bot_record(
    tx: &mut Tx<'_>,
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
        "bots_create" => create_bot(tx, resolved, &bot_id, &item.content, now).await,
        "bots_delete" => set_deleted(tx, resolved, &bot_id, true).await,
        "bots_restore" => set_deleted(tx, resolved, &bot_id, false).await,
        other => Err(denied_detail(
            StatusCode::BAD_REQUEST,
            &format!("unknown bots kind: {other}"),
        )),
    }
}

async fn set_deleted(
    tx: &mut Tx<'_>,
    resolved: &ResolvedStream,
    bot_id: &str,
    deleted: bool,
) -> Result<(), CallerDenied> {
    streams_projection::set_owned_bot_deleted_in(tx, bot_id, &resolved.owner, deleted)
        .await
        .map(drop)
        .map_err(internal_sqlx)
}

async fn create_bot(
    tx: &mut Tx<'_>,
    resolved: &ResolvedStream,
    bot_id: &str,
    content: &serde_json::Value,
    now: i64,
) -> Result<(), CallerDenied> {
    let name = content_name(content)?;
    streams_projection::ensure_bot_in(tx, bot_id, &resolved.owner, &name, now)
        .await
        .map_err(internal_sqlx)?;
    let key = selector_key(
        &StreamSelector::Bot {
            id: bot_id.to_owned(),
        },
        &resolved.owner,
    );
    let kind = format!("bot:{bot_id}");
    streams_projection::ensure_stream_in(tx, &key, &kind, &resolved.owner, Some(bot_id), now)
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
