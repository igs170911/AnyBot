use std::sync::Arc;

use anybot_domain::window::count_in_window;
use anybot_foundation::{Clock, TimestampMs};
use anybot_store_server::{Pool, auth_events};

pub const KIND_LOGIN_FAIL: &str = "login-fail";
pub const KIND_REGISTER: &str = "register";
pub const KIND_PASSWORD_CHANGE: &str = "password-change";
pub const PAIR_LIMIT: usize = 8;
pub const ACCOUNT_LIMIT: usize = 40;
pub const REGISTER_LIMIT: usize = 5;
pub const PASSWORD_LIMIT: usize = 8;

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

    pub async fn login_blocked(&self, address: &str, account: &str) -> Result<bool, sqlx::Error> {
        let pair = auth_events::auth_events_for_pair(&self.pool, KIND_LOGIN_FAIL, address, account)
            .await?;
        let per_account =
            auth_events::auth_events_for_account(&self.pool, KIND_LOGIN_FAIL, account).await?;
        let now_ms = self.now_ms();
        let pair_u = to_u64(&pair)?;
        let account_u = to_u64(&per_account)?;
        let span = anybot_domain::window::WINDOW_SPAN_MS;
        Ok(count_in_window(&pair_u, now_ms, span) >= PAIR_LIMIT
            || count_in_window(&account_u, now_ms, span) >= ACCOUNT_LIMIT)
    }

    pub async fn register_blocked(&self, address: &str) -> Result<bool, sqlx::Error> {
        let events =
            auth_events::auth_events_for_address(&self.pool, KIND_REGISTER, address).await?;
        let events_u = to_u64(&events)?;
        Ok(count_in_window(
            &events_u,
            self.now_ms(),
            anybot_domain::window::WINDOW_SPAN_MS,
        ) >= REGISTER_LIMIT)
    }

    pub async fn password_blocked(&self, account: &str) -> Result<bool, sqlx::Error> {
        let events =
            auth_events::auth_events_for_account(&self.pool, KIND_PASSWORD_CHANGE, account).await?;
        let events_u = to_u64(&events)?;
        Ok(count_in_window(
            &events_u,
            self.now_ms(),
            anybot_domain::window::WINDOW_SPAN_MS,
        ) >= PASSWORD_LIMIT)
    }

    pub async fn record(
        &self,
        kind: &str,
        address: &str,
        account: &str,
    ) -> Result<(), sqlx::Error> {
        let at_ms =
            i64::try_from(self.now_ms()).map_err(|e| sqlx::Error::Protocol(e.to_string()))?;
        auth_events::record_auth_event(&self.pool, kind, address, account, at_ms).await
    }

    fn now_ms(&self) -> u64 {
        self.clock.now_ms().millis()
    }
}

fn to_u64(events: &[i64]) -> Result<Vec<u64>, sqlx::Error> {
    let mut out = Vec::with_capacity(events.len());
    for at in events {
        out.push(u64::try_from(*at).map_err(|e| sqlx::Error::Protocol(e.to_string()))?);
    }
    Ok(out)
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
