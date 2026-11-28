use anybot_foundation::Secret;

use crate::Pool;
use crate::push_key::PushStoreError;
use crate::secret_keys::SecretKeys;
use crate::secret_seal::SecretPlace;

pub const SUBSCRIPTION_TABLE: &str = "push_subscriptions";
pub const SUBSCRIPTION_AUTH_COLUMN: &str = "auth_sealed";
pub const SUBSCRIPTIONS_PER_ACCOUNT: i64 = 16;

#[must_use]
pub fn auth_place(id: &str) -> SecretPlace<'_> {
    SecretPlace {
        table: SUBSCRIPTION_TABLE,
        column: SUBSCRIPTION_AUTH_COLUMN,
        row: id,
    }
}

pub struct NewSubscription<'a> {
    pub id: &'a str,
    pub account: &'a str,
    pub device_id: &'a str,
    pub endpoint: &'a str,
    pub p256dh: &'a str,
    pub auth: &'a Secret,
    pub created_at_ms: i64,
}

#[derive(Debug)]
pub struct StoredSubscription {
    pub id: String,
    pub account: String,
    pub device_id: String,
    pub endpoint: String,
    pub p256dh: String,
    pub auth: Secret,
}

type Tx<'a> = sqlx::Transaction<'a, sqlx::Postgres>;

async fn lock_account(tx: &mut Tx<'_>, account: &str) -> Result<(), sqlx::Error> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtext($1)::bigint)")
        .bind(account)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

async fn insert_row(
    tx: &mut Tx<'_>,
    new: &NewSubscription<'_>,
    sealed: &[u8],
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "DELETE FROM push_subscriptions WHERE endpoint = $1",
        new.endpoint
    )
    .execute(&mut **tx)
    .await?;
    sqlx::query!(
        "INSERT INTO push_subscriptions \
         (id, account, device_id, endpoint, p256dh, auth_sealed, created_at_ms) \
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
        new.id,
        new.account,
        new.device_id,
        new.endpoint,
        new.p256dh,
        sealed,
        new.created_at_ms,
    )
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn trim_to_cap(tx: &mut Tx<'_>, account: &str) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "DELETE FROM push_subscriptions WHERE id IN ( \
           SELECT id FROM push_subscriptions WHERE account = $1 \
           ORDER BY created_at_ms DESC, id DESC OFFSET $2)",
        account,
        SUBSCRIPTIONS_PER_ACCOUNT,
    )
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub async fn upsert(
    pool: &Pool,
    keys: &SecretKeys,
    new: &NewSubscription<'_>,
) -> Result<(), PushStoreError> {
    let sealed = keys.seal_text(&auth_place(new.id), new.auth)?;
    let mut tx = pool.begin().await?;
    lock_account(&mut tx, new.account).await?;
    insert_row(&mut tx, new, &sealed).await?;
    trim_to_cap(&mut tx, new.account).await?;
    tx.commit().await?;
    Ok(())
}

pub async fn delete_owned(pool: &Pool, account: &str, id: &str) -> Result<bool, sqlx::Error> {
    let done = sqlx::query!(
        "DELETE FROM push_subscriptions WHERE id = $1 AND account = $2",
        id,
        account
    )
    .execute(pool)
    .await?;
    Ok(done.rows_affected() == 1)
}

pub async fn delete_gone(pool: &Pool, id: &str) -> Result<bool, sqlx::Error> {
    let done = sqlx::query!("DELETE FROM push_subscriptions WHERE id = $1", id)
        .execute(pool)
        .await?;
    Ok(done.rows_affected() == 1)
}

pub async fn exists_for_device(
    pool: &Pool,
    account: &str,
    device_id: &str,
) -> Result<bool, sqlx::Error> {
    let found = sqlx::query_scalar!(
        "SELECT COUNT(*) AS \"n!\" FROM push_subscriptions WHERE account = $1 AND device_id = $2",
        account,
        device_id,
    )
    .fetch_one(pool)
    .await?;
    Ok(found > 0)
}

pub async fn list_for_account(
    pool: &Pool,
    keys: &SecretKeys,
    account: &str,
) -> Result<Vec<StoredSubscription>, PushStoreError> {
    let rows = sqlx::query!(
        "SELECT id, account, device_id, endpoint, p256dh, auth_sealed \
         FROM push_subscriptions WHERE account = $1 ORDER BY created_at_ms, id",
        account,
    )
    .fetch_all(pool)
    .await?;
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let auth = keys.open_text(&auth_place(&row.id), &row.auth_sealed)?;
        out.push(StoredSubscription {
            id: row.id,
            account: row.account,
            device_id: row.device_id,
            endpoint: row.endpoint,
            p256dh: row.p256dh,
            auth,
        });
    }
    Ok(out)
}
