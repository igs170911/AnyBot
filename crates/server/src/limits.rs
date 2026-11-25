use std::sync::Arc;

use anybot_domain::window::WINDOW_SPAN_MS;
use anybot_foundation::{Clock, TimestampMs};
use anybot_store_server::{Pool, auth_events};

pub const KIND_LOGIN_FAIL: &str = "login-fail";
pub const KIND_REGISTER: &str = "register";
pub const KIND_PASSWORD_CHANGE: &str = "password-change";
pub const PAIR_LIMIT: i64 = 8;
pub const ACCOUNT_LIMIT: i64 = 40;
pub const REGISTER_LIMIT: i64 = 5;
pub const PASSWORD_LIMIT: i64 = 8;

#[derive(Clone)]
pub struct Limiter {
    pool: Pool,
    clock: Arc<dyn Clock>,
}

impl Limiter {
    #[must_use]
    pub fn new(pool: &Pool, clock: Arc<dyn Clock>) -> Self {
        Limiter {
            pool: pool.clone(),
            clock,
        }
    }

    pub async fn begin_login(
        &self,
        address: &str,
        account: &str,
    ) -> Result<Option<i64>, sqlx::Error> {
        let floor = self.floor()?;
        let at = self.now_i64()?;
        let mut tx = self.pool.begin().await?;
        let id =
            auth_events::begin_attempt(&mut *tx, KIND_LOGIN_FAIL, address, account, at).await?;
        let pair = auth_events::count_pair(&mut *tx, KIND_LOGIN_FAIL, address, account, floor).await?;
        let total = auth_events::count_account(&mut *tx, KIND_LOGIN_FAIL, account, floor).await?;
        if pair > PAIR_LIMIT || total > ACCOUNT_LIMIT {
            tx.rollback().await?;
            return Ok(None);
        }
        tx.commit().await?;
        Ok(Some(id))
    }

    pub async fn begin_register(&self, address: &str) -> Result<Option<i64>, sqlx::Error> {
        let floor = self.floor()?;
        let at = self.now_i64()?;
        let mut tx = self.pool.begin().await?;
        let id = auth_events::begin_attempt(&mut *tx, KIND_REGISTER, address, "", at).await?;
        let total = auth_events::count_address(&mut *tx, KIND_REGISTER, address, floor).await?;
        if total > REGISTER_LIMIT {
            tx.rollback().await?;
            return Ok(None);
        }
        tx.commit().await?;
        Ok(Some(id))
    }

    pub async fn cancel_login(&self, id: i64) -> Result<(), sqlx::Error> {
        auth_events::drop_attempt(&self.pool, id).await
    }

    fn floor(&self) -> Result<i64, sqlx::Error> {
        events_before(self.clock.now_ms(), WINDOW_SPAN_MS)
    }

    fn now_i64(&self) -> Result<i64, sqlx::Error> {
        i64::try_from(self.now_ms()).map_err(|e| sqlx::Error::Protocol(e.to_string()))
    }

    pub async fn password_blocked(&self, account: &str) -> Result<bool, sqlx::Error> {
        let floor = self.floor()?;
        let total =
            auth_events::count_account(&self.pool, KIND_PASSWORD_CHANGE, account, floor).await?;
        Ok(total >= PASSWORD_LIMIT)
    }

    pub async fn record(
        &self,
        kind: &str,
        address: &str,
        account: &str,
    ) -> Result<(), sqlx::Error> {
        let at_ms = self.now_i64()?;
        auth_events::record_auth_event(&self.pool, kind, address, account, at_ms).await
    }

    fn now_ms(&self) -> u64 {
        self.clock.now_ms().millis()
    }
}

pub fn events_before(now: TimestampMs, span_ms: u64) -> Result<i64, sqlx::Error> {
    i64::try_from(now.millis().saturating_sub(span_ms))
        .map_err(|e| sqlx::Error::Protocol(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prune_floor_is_the_window_start() {
        let now = TimestampMs::from_millis(500_000);
        assert_eq!(events_before(now, 300_000).ok(), Some(200_000));
    }
}
