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

pub async fn begin_attempt<'q, E>(
    executor: E,
    kind: &str,
    address: &str,
    account: &str,
    at_ms: i64,
) -> Result<i64, sqlx::Error>
where
    E: sqlx::Executor<'q, Database = sqlx::Postgres>,
{
    let row = sqlx::query!(
        "INSERT INTO auth_events (kind, address, account, at_ms) VALUES ($1, $2, $3, $4) RETURNING id",
        kind,
        address,
        account,
        at_ms,
    )
    .fetch_one(executor)
    .await?;
    Ok(row.id)
}

pub async fn count_pair<'q, E>(
    executor: E,
    kind: &str,
    address: &str,
    account: &str,
    floor_ms: i64,
) -> Result<i64, sqlx::Error>
where
    E: sqlx::Executor<'q, Database = sqlx::Postgres>,
{
    let row = sqlx::query!(
        "SELECT count(*) AS total FROM auth_events WHERE kind = $1 AND address = $2 AND account = $3 AND at_ms >= $4",
        kind,
        address,
        account,
        floor_ms,
    )
    .fetch_one(executor)
    .await?;
    row.total.ok_or(sqlx::Error::ColumnNotFound("total".to_owned()))
}

pub async fn count_account<'q, E>(
    executor: E,
    kind: &str,
    account: &str,
    floor_ms: i64,
) -> Result<i64, sqlx::Error>
where
    E: sqlx::Executor<'q, Database = sqlx::Postgres>,
{
    let row = sqlx::query!(
        "SELECT count(*) AS total FROM auth_events WHERE kind = $1 AND account = $2 AND at_ms >= $3",
        kind,
        account,
        floor_ms,
    )
    .fetch_one(executor)
    .await?;
    row.total.ok_or(sqlx::Error::ColumnNotFound("total".to_owned()))
}

pub async fn count_address<'q, E>(
    executor: E,
    kind: &str,
    address: &str,
    floor_ms: i64,
) -> Result<i64, sqlx::Error>
where
    E: sqlx::Executor<'q, Database = sqlx::Postgres>,
{
    let row = sqlx::query!(
        "SELECT count(*) AS total FROM auth_events WHERE kind = $1 AND address = $2 AND at_ms >= $3",
        kind,
        address,
        floor_ms,
    )
    .fetch_one(executor)
    .await?;
    row.total.ok_or(sqlx::Error::ColumnNotFound("total".to_owned()))
}

pub async fn drop_attempt(pool: &Pool, id: i64) -> Result<(), sqlx::Error> {
    sqlx::query!("DELETE FROM auth_events WHERE id = $1", id)
        .execute(pool)
        .await?;
    Ok(())
}
