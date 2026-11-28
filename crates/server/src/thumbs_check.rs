use serde_json::Value;

use anybot_messages::MessageCode;
use anybot_protocol::RecordItem;
use anybot_store_server::streams_task;

use crate::api_streams_util::ResolvedStream;

pub const THUMBS_UP_KIND: &str = "thumbs_up";
pub const THUMBS_REMOVED_KIND: &str = "thumbs_up_removed";
pub const ANSWER_KIND: &str = "model_answer";

#[must_use]
pub fn is_thumbs_kind(kind: &str) -> bool {
    kind == THUMBS_UP_KIND || kind == THUMBS_REMOVED_KIND
}

pub async fn thumbs_accepted(
    pool: &anybot_store_server::Pool,
    resolved: &ResolvedStream,
    item: &RecordItem,
) -> Result<bool, String> {
    if resolved.bot.is_none() {
        return Ok(false);
    }
    let Some(task_id) = item.task_id.as_deref() else {
        return Ok(false);
    };
    let Some(answer_seq) = answer_seq_of(&item.content) else {
        return Ok(false);
    };
    let seq = i64::try_from(answer_seq).map_err(|e| e.to_string())?;
    let found = streams_task::answer_record_exists(pool, &resolved.key, task_id, seq, ANSWER_KIND)
        .await
        .map_err(|e| e.to_string())?;
    Ok(found)
}

fn answer_seq_of(content: &Value) -> Option<u64> {
    content.get("answer_seq").and_then(Value::as_u64)
}

#[must_use]
pub fn thumbs_denial_code() -> MessageCode {
    MessageCode::ThumbsTargetInvalid
}
