use crate::Pool;

pub async fn record_auth_event(
    pool: &Pool,
    kind: &str,
    address: &str,
    account: &str,
    at_ms: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "INSERT INTO auth_events (kind, address, account, at_ms) VALUES ($1, $2, $3, $4)",
        kind,
        address,
        account,
        at_ms,
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn auth_events_for_pair(
    pool: &Pool,
    kind: &str,
    address: &str,
    account: &str,
) -> Result<Vec<i64>, sqlx::Error> {
    let rows = sqlx::query!(
        "SELECT at_ms FROM auth_events WHERE kind = $1 AND address = $2 AND account = $3",
        kind,
        address,
        account,
    )
    .fetch_all(pool)
    .await?;
    Ok(rows.iter().map(|row| row.at_ms).collect())
}

pub async fn auth_events_for_account(
    pool: &Pool,
    kind: &str,
    account: &str,
) -> Result<Vec<i64>, sqlx::Error> {
    let rows = sqlx::query!(
        "SELECT at_ms FROM auth_events WHERE kind = $1 AND account = $2",
        kind,
        account,
    )
    .fetch_all(pool)
    .await?;
    Ok(rows.iter().map(|row| row.at_ms).collect())
}

pub async fn auth_events_for_address(
    pool: &Pool,
    kind: &str,
    address: &str,
) -> Result<Vec<i64>, sqlx::Error> {
    let rows = sqlx::query!(
        "SELECT at_ms FROM auth_events WHERE kind = $1 AND address = $2",
        kind,
        address,
    )
    .fetch_all(pool)
    .await?;
    Ok(rows.iter().map(|row| row.at_ms).collect())
}

pub async fn prune_auth_events_before(pool: &Pool, floor_ms: i64) -> Result<u64, sqlx::Error> {
    let result = sqlx::query!("DELETE FROM auth_events WHERE at_ms < $1", floor_ms)
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}
