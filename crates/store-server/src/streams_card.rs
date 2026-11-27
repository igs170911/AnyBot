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
