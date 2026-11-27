pub async fn count_kind<'q, E>(executor: E, kind: &str, floor_ms: i64) -> Result<i64, sqlx::Error>
where
    E: sqlx::Executor<'q, Database = sqlx::Postgres>,
{
    let row = sqlx::query!(
        "SELECT count(*) AS total FROM auth_events WHERE kind = $1 AND at_ms >= $2",
        kind,
        floor_ms,
    )
    .fetch_one(executor)
    .await?;
    row.total
        .ok_or(sqlx::Error::ColumnNotFound("total".to_owned()))
}
