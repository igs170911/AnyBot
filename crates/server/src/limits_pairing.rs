use anybot_domain::pairing_rules::{ADDRESS_FAIL_LIMIT, GLOBAL_FAIL_LIMIT};
use anybot_store_server::{auth_events, auth_events_kind};

use crate::limits::Limiter;

pub const KIND_PAIR_FAIL: &str = "pair-fail";
const GLOBAL_LOCK: &str = "*";

impl Limiter {
    pub async fn begin_pairing(&self, address: &str) -> Result<Option<i64>, sqlx::Error> {
        let floor = self.floor()?;
        let at = self.now_i64()?;
        let mut tx = self.pool.begin().await?;
        auth_events::lock_counter(&mut tx, KIND_PAIR_FAIL, GLOBAL_LOCK, "").await?;
        let id = auth_events::begin_attempt(&mut *tx, KIND_PAIR_FAIL, address, "", at).await?;
        let by_address =
            auth_events::count_address(&mut *tx, KIND_PAIR_FAIL, address, floor).await?;
        let total = auth_events_kind::count_kind(&mut *tx, KIND_PAIR_FAIL, floor).await?;
        if by_address > ADDRESS_FAIL_LIMIT || total > GLOBAL_FAIL_LIMIT {
            tx.rollback().await?;
            return Ok(None);
        }
        tx.commit().await?;
        Ok(Some(id))
    }
}
