use sqlx::{Postgres, Transaction};

use crate::Pool;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LetterRow {
    pub letter_id: String,
    pub account: String,
    pub kind: String,
    pub device_id: String,
    pub content: String,
    pub created_at_ms: i64,
    pub delivered_at_ms: Option<i64>,
    pub state: String,
    pub claimed_by: Option<String>,
}

pub struct NewLetter<'a> {
    pub letter_id: &'a str,
    pub account: &'a str,
    pub kind: &'a str,
    pub device_id: &'a str,
    pub content: &'a str,
    pub now_ms: i64,
}

pub async fn insert_in(
    tx: &mut Transaction<'_, Postgres>,
    letter: &NewLetter<'_>,
) -> Result<bool, sqlx::Error> {
    let done = sqlx::query!(
        "INSERT INTO mailbox_letters (letter_id, account, kind, device_id, content, \
         created_at_ms, state) VALUES ($1, $2, $3, $4, $5, $6, 'pending') \
         ON CONFLICT (letter_id) DO NOTHING",
        letter.letter_id,
        letter.account,
        letter.kind,
        letter.device_id,
        letter.content,
        letter.now_ms,
    )
    .execute(&mut **tx)
    .await?;
    Ok(done.rows_affected() == 1)
}

pub async fn insert(pool: &Pool, letter: &NewLetter<'_>) -> Result<bool, sqlx::Error> {
    let mut tx = pool.begin().await?;
    let inserted = insert_in(&mut tx, letter).await?;
    tx.commit().await?;
    Ok(inserted)
}

pub async fn find(pool: &Pool, letter_id: &str) -> Result<Option<LetterRow>, sqlx::Error> {
    sqlx::query_as!(
        LetterRow,
        "SELECT letter_id, account, kind, device_id, content, created_at_ms, delivered_at_ms, \
         state, claimed_by FROM mailbox_letters WHERE letter_id = $1",
        letter_id,
    )
    .fetch_optional(pool)
    .await
}

pub async fn claim(
    pool: &Pool,
    letter_id: &str,
    device_id: &str,
    now_ms: i64,
) -> Result<bool, sqlx::Error> {
    let done = sqlx::query!(
        "UPDATE mailbox_letters SET state = 'claimed', claimed_by = $2, claimed_at_ms = $3 \
         WHERE letter_id = $1 AND device_id = $2 AND state = 'pending'",
        letter_id,
        device_id,
        now_ms,
    )
    .execute(pool)
    .await?;
    Ok(done.rows_affected() == 1)
}

pub async fn pending_for_device(
    pool: &Pool,
    device_id: &str,
) -> Result<Vec<LetterRow>, sqlx::Error> {
    sqlx::query_as!(
        LetterRow,
        "SELECT letter_id, account, kind, device_id, content, created_at_ms, delivered_at_ms, \
         state, claimed_by FROM mailbox_letters \
         WHERE device_id = $1 AND state = 'pending' ORDER BY created_at_ms, letter_id",
        device_id,
    )
    .fetch_all(pool)
    .await
}

pub async fn mark_delivered(pool: &Pool, letter_id: &str, now_ms: i64) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE mailbox_letters SET delivered_at_ms = $2 \
         WHERE letter_id = $1 AND delivered_at_ms IS NULL AND state = 'pending'",
        letter_id,
        now_ms,
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn expired_rounds(
    pool: &Pool,
    bot_computer: &str,
    delivered_floor_ms: i64,
) -> Result<Vec<LetterRow>, sqlx::Error> {
    sqlx::query_as!(
        LetterRow,
        "SELECT letter_id, account, kind, device_id, content, created_at_ms, delivered_at_ms, \
         state, claimed_by FROM mailbox_letters \
         WHERE state = 'pending' AND kind = 'round' AND device_id <> $1 \
         AND COALESCE(delivered_at_ms, created_at_ms) <= $2 \
         ORDER BY created_at_ms, letter_id",
        bot_computer,
        delivered_floor_ms,
    )
    .fetch_all(pool)
    .await
}

pub async fn rounds_for_gone_computers(pool: &Pool) -> Result<Vec<LetterRow>, sqlx::Error> {
    sqlx::query_as!(
        LetterRow,
        "SELECT l.letter_id, l.account, l.kind, l.device_id, l.content, l.created_at_ms, \
         l.delivered_at_ms, l.state, l.claimed_by FROM mailbox_letters l \
         JOIN computers c ON c.device_id = l.device_id \
         WHERE l.state = 'pending' AND l.kind = 'round' \
         AND (c.online = FALSE OR c.removed_at_ms IS NOT NULL) \
         ORDER BY l.created_at_ms, l.letter_id",
    )
    .fetch_all(pool)
    .await
}

pub async fn pending_rounds_of(
    pool: &Pool,
    device_id: &str,
) -> Result<Vec<LetterRow>, sqlx::Error> {
    sqlx::query_as!(
        LetterRow,
        "SELECT letter_id, account, kind, device_id, content, created_at_ms, delivered_at_ms, \
         state, claimed_by FROM mailbox_letters \
         WHERE device_id = $1 AND state = 'pending' AND kind = 'round' \
         ORDER BY created_at_ms, letter_id",
        device_id,
    )
    .fetch_all(pool)
    .await
}

pub async fn mark_reassigned_in(
    tx: &mut Transaction<'_, Postgres>,
    letter_id: &str,
    to_letter: &str,
    reason: &str,
    now_ms: i64,
) -> Result<bool, sqlx::Error> {
    let done = sqlx::query!(
        "UPDATE mailbox_letters SET state = 'reassigned', reassigned_to = $2, \
         reassign_reason = $3, closed_at_ms = $4 WHERE letter_id = $1 AND state = 'pending'",
        letter_id,
        to_letter,
        reason,
        now_ms,
    )
    .execute(&mut **tx)
    .await?;
    Ok(done.rows_affected() == 1)
}

pub async fn retract_cards_of(
    pool: &Pool,
    device_id: &str,
    now_ms: i64,
) -> Result<u64, sqlx::Error> {
    let done = sqlx::query!(
        "UPDATE mailbox_letters SET state = 'retracted', closed_at_ms = $2 \
         WHERE device_id = $1 AND state = 'pending' AND kind = 'card-answer'",
        device_id,
        now_ms,
    )
    .execute(pool)
    .await?;
    Ok(done.rows_affected())
}

pub async fn prune_closed(pool: &Pool, closed_floor_ms: i64) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "DELETE FROM mailbox_letters WHERE state IN ('reassigned', 'retracted') \
         AND closed_at_ms < $1",
        closed_floor_ms,
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn note_issuer(
    pool: &Pool,
    stream_id: &str,
    card_id: &str,
    device_id: &str,
    now_ms: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "INSERT INTO card_issuers (stream_id, card_id, device_id, noted_at_ms) \
         VALUES ($1, $2, $3, $4) ON CONFLICT (stream_id, card_id) DO NOTHING",
        stream_id,
        card_id,
        device_id,
        now_ms,
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn issuer_of(
    pool: &Pool,
    stream_id: &str,
    card_id: &str,
) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_scalar!(
        "SELECT device_id FROM card_issuers WHERE stream_id = $1 AND card_id = $2",
        stream_id,
        card_id,
    )
    .fetch_optional(pool)
    .await
}
