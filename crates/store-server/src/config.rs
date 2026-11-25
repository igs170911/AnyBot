use sqlx::Row;

use crate::Pool;

pub async fn read_config_rows(pool: &Pool) -> Result<Vec<(String, String)>, sqlx::Error> {
    let rows = sqlx::query("SELECT key, value FROM server_config ORDER BY key")
        .fetch_all(pool)
        .await?;
    let mut out = Vec::new();
    for row in rows {
        let key: String = row.try_get("key")?;
        let value: String = row.try_get("value")?;
        out.push((key, value));
    }
    Ok(out)
}

pub async fn write_config_row(pool: &Pool, key: &str, value: &str) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO server_config (key, value) VALUES ($1, $2) \
         ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value",
    )
    .bind(key)
    .bind(value)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn delete_config_row(pool: &Pool, key: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM server_config WHERE key = $1")
        .bind(key)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn write_config_rows(pool: &Pool, rows: &[(String, String)]) -> Result<(), sqlx::Error> {
    for (key, value) in rows {
        write_config_row(pool, key, value).await?;
    }
    Ok(())
}

pub async fn clear_config_keys(pool: &Pool, keys: &[&str]) -> Result<(), sqlx::Error> {
    for key in keys {
        delete_config_row(pool, key).await?;
    }
    Ok(())
}

pub async fn set_config_rows(
    pool: &Pool,
    rows: &[(String, String)],
    keys: &[&str],
) -> Result<(), sqlx::Error> {
    clear_config_keys(pool, keys).await?;
    write_config_rows(pool, rows).await
}
