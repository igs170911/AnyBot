use sqlx::{Postgres, Transaction};

pub struct CardClaim<'a> {
    pub stream_id: &'a str,
    pub card_id: &'a str,
    pub device_id: &'a str,
    pub answer: &'a str,
    pub seq: i64,
}

pub async fn claim_card_in(
    tx: &mut Transaction<'_, Postgres>,
    claim: &CardClaim<'_>,
) -> Result<Option<String>, sqlx::Error> {
    let claimed = sqlx::query!(
        "INSERT INTO card_answers (stream_id, card_id, device_id, answer, seq) \
         VALUES ($1, $2, $3, $4, $5) ON CONFLICT (stream_id, card_id) DO NOTHING \
         RETURNING device_id",
        claim.stream_id,
        claim.card_id,
        claim.device_id,
        claim.answer,
        claim.seq,
    )
    .fetch_optional(&mut **tx)
    .await?;
    if claimed.is_some() {
        return Ok(None);
    }
    let earlier = sqlx::query!(
        "SELECT device_id FROM card_answers WHERE stream_id = $1 AND card_id = $2",
        claim.stream_id,
        claim.card_id,
    )
    .fetch_one(&mut **tx)
    .await?;
    Ok(Some(earlier.device_id))
}

pub async fn answered_by_in(
    tx: &mut Transaction<'_, Postgres>,
    stream_id: &str,
    card_id: &str,
) -> Result<Option<String>, sqlx::Error> {
    let row = sqlx::query!(
        "SELECT device_id FROM card_answers WHERE stream_id = $1 AND card_id = $2",
        stream_id,
        card_id,
    )
    .fetch_optional(&mut **tx)
    .await?;
    Ok(row.map(|r| r.device_id))
}

#[derive(Debug, Clone)]
pub struct TaskCardRecord {
    pub kind: String,
    pub content: String,
}

pub async fn task_card_records_in(
    tx: &mut Transaction<'_, Postgres>,
    stream_id: &str,
    task_id: &str,
    kinds: &[String],
) -> Result<Vec<TaskCardRecord>, sqlx::Error> {
    let rows = sqlx::query!(
        "SELECT kind, content FROM records WHERE stream_id = $1 AND task_id = $2 \
         AND kind = ANY($3) ORDER BY seq",
        stream_id,
        task_id,
        kinds,
    )
    .fetch_all(&mut **tx)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| TaskCardRecord {
            kind: r.kind,
            content: r.content,
        })
        .collect())
}
