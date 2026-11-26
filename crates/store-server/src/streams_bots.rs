use crate::Pool;

#[derive(Debug, Clone)]
pub struct BotRow {
    pub id: String,
    pub account: String,
    pub name: String,
    pub created_at_ms: i64,
    pub deleted: bool,
}

pub async fn load_bot(pool: &Pool, id: &str) -> Result<Option<BotRow>, sqlx::Error> {
    let row = sqlx::query!(
        "SELECT id, account, name, created_at_ms, deleted FROM bots WHERE id = $1",
        id,
    )
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|r| BotRow {
        id: r.id,
        account: r.account,
        name: r.name,
        created_at_ms: r.created_at_ms,
        deleted: r.deleted,
    }))
}

pub async fn ensure_bot(
    pool: &Pool,
    id: &str,
    account: &str,
    name: &str,
    created_at_ms: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "INSERT INTO bots (id, account, name, created_at_ms) VALUES ($1, $2, $3, $4) \
         ON CONFLICT (id) DO NOTHING",
        id,
        account,
        name,
        created_at_ms,
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn set_bot_deleted(pool: &Pool, id: &str, deleted: bool) -> Result<(), sqlx::Error> {
    sqlx::query!("UPDATE bots SET deleted = $2 WHERE id = $1", id, deleted,)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn rename_bot(pool: &Pool, id: &str, name: &str) -> Result<(), sqlx::Error> {
    sqlx::query!("UPDATE bots SET name = $2 WHERE id = $1", id, name)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn list_bots(pool: &Pool, account: &str) -> Result<Vec<BotRow>, sqlx::Error> {
    let rows = sqlx::query!(
        "SELECT id, account, name, created_at_ms, deleted FROM bots WHERE account = $1 \
         ORDER BY created_at_ms, id",
        account,
    )
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| BotRow {
            id: r.id,
            account: r.account,
            name: r.name,
            created_at_ms: r.created_at_ms,
            deleted: r.deleted,
        })
        .collect())
}
