use std::time::Duration;

use sqlx::pool::PoolOptions;
use sqlx::postgres::PgConnectOptions;

use anybot_foundation::ServerBootstrap;

use crate::Pool;

pub const ACQUIRE_TIMEOUT: Duration = Duration::from_secs(30);
pub const MAX_CONNECTIONS: u32 = 8;

#[must_use]
pub fn options(bootstrap: &ServerBootstrap) -> PgConnectOptions {
    PgConnectOptions::new()
        .host(&bootstrap.host)
        .port(bootstrap.port)
        .username(&bootstrap.user)
        .password(bootstrap.password.expose())
        .database(&bootstrap.database)
}

#[must_use]
pub fn pool_options() -> PoolOptions<sqlx::Postgres> {
    PoolOptions::new()
        .acquire_timeout(ACQUIRE_TIMEOUT)
        .max_connections(MAX_CONNECTIONS)
}

pub const PROBE_ACQUIRE_TIMEOUT: Duration = Duration::from_secs(2);

#[must_use]
pub fn open_lazy(bootstrap: &ServerBootstrap) -> Pool {
    pool_options().connect_lazy_with(options(bootstrap))
}

#[must_use]
pub fn probe_pool(bootstrap: &ServerBootstrap) -> Pool {
    PoolOptions::new()
        .acquire_timeout(PROBE_ACQUIRE_TIMEOUT)
        .max_connections(1)
        .connect_lazy_with(options(bootstrap))
}

pub async fn probe(pool: &Pool) -> Result<u32, sqlx::Error> {
    sqlx::query!("SELECT 1 AS one").fetch_one(pool).await?;
    Ok(1)
}

pub fn open_url_lazy(url: &str) -> Result<Pool, sqlx::Error> {
    let parsed = url
        .parse::<PgConnectOptions>()
        .map_err(|e| sqlx::Error::Configuration(e.into()))?;
    Ok(pool_options().connect_lazy_with(parsed))
}

pub async fn open_url(url: &str) -> Result<Pool, sqlx::Error> {
    let parsed = url
        .parse::<PgConnectOptions>()
        .map_err(|e| sqlx::Error::Configuration(e.into()))?;
    pool_options().connect_with(parsed).await
}
