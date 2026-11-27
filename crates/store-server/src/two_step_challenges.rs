use crate::Pool;

pub const CHALLENGE_OPEN: &str = "open";
pub const CHALLENGE_PASSED: &str = "passed";
pub const CHALLENGE_VOID: &str = "void";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChallengeRow {
    pub account: String,
    pub credential: String,
    pub label: String,
    pub created_at_ms: i64,
    pub tries: i32,
    pub state: String,
    pub passed_at_ms: Option<i64>,
    pub used_recovery: bool,
}

pub struct NewChallenge<'a> {
    pub id_hash: &'a [u8],
    pub account: &'a str,
    pub credential: &'a str,
    pub label: &'a str,
    pub at_ms: i64,
}

pub async fn insert_challenge(pool: &Pool, new: &NewChallenge<'_>) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "INSERT INTO two_step_challenges (id_hash, account, credential, label, created_at_ms) \
         VALUES ($1, $2, $3, $4, $5)",
        new.id_hash,
        new.account,
        new.credential,
        new.label,
        new.at_ms,
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn find_challenge(
    pool: &Pool,
    id_hash: &[u8],
) -> Result<Option<ChallengeRow>, sqlx::Error> {
    let row = sqlx::query!(
        "SELECT account, credential, label, created_at_ms, tries, state, passed_at_ms, \
         used_recovery FROM two_step_challenges WHERE id_hash = $1",
        id_hash
    )
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|row| ChallengeRow {
        account: row.account,
        credential: row.credential,
        label: row.label,
        created_at_ms: row.created_at_ms,
        tries: row.tries,
        state: row.state,
        passed_at_ms: row.passed_at_ms,
        used_recovery: row.used_recovery,
    }))
}

pub async fn take_try(
    pool: &Pool,
    id_hash: &[u8],
    max_tries: i32,
) -> Result<Option<i32>, sqlx::Error> {
    sqlx::query_scalar!(
        "UPDATE two_step_challenges SET tries = tries + 1 \
         WHERE id_hash = $1 AND state = 'open' AND tries < $2 RETURNING tries",
        id_hash,
        max_tries,
    )
    .fetch_optional(pool)
    .await
}

pub async fn close_challenge(
    pool: &Pool,
    id_hash: &[u8],
    state: &str,
    at_ms: i64,
    used_recovery: bool,
) -> Result<bool, sqlx::Error> {
    let done = sqlx::query!(
        "UPDATE two_step_challenges SET state = $2, passed_at_ms = $3, used_recovery = $4 \
         WHERE id_hash = $1 AND state = 'open'",
        id_hash,
        state,
        at_ms,
        used_recovery,
    )
    .execute(pool)
    .await?;
    Ok(done.rows_affected() == 1)
}

pub async fn prune_challenges_before(pool: &Pool, floor_ms: i64) -> Result<u64, sqlx::Error> {
    let done = sqlx::query!(
        "DELETE FROM two_step_challenges WHERE created_at_ms < $1",
        floor_ms
    )
    .execute(pool)
    .await?;
    Ok(done.rows_affected())
}
