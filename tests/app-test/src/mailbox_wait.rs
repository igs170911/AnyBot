use std::time::Duration;

use crate::mailbox_prop::Rig;

const STEP_REAL: Duration = Duration::from_millis(20);
const NUDGE_EVERY_STEPS: u32 = 8;
const NUDGE_FAKE: Duration = Duration::from_secs(5);
const WAIT_LIMIT: Duration = Duration::from_secs(30);

impl Rig<'_> {
    pub(crate) async fn count(&self, sql: &'static str, binds: &[&str]) -> Result<i64, String> {
        let mut query = sqlx::query_scalar::<_, i64>(sql);
        for bind in binds {
            query = query.bind(*bind);
        }
        query
            .fetch_one(&self.served.pool)
            .await
            .map_err(|e| e.to_string())
    }

    pub(crate) async fn wait_until_flagged_offline(&self) -> Result<(), String> {
        let a = self.seats.a.device_id.clone();
        let b = self.seats.b.device_id.clone();
        let sql = "SELECT count(*) FROM computers WHERE online = FALSE AND device_id IN ($1, $2)";
        let wait = async {
            let mut steps = 0u32;
            loop {
                if self.count(sql, &[&a, &b]).await? == 2 {
                    return Ok::<(), String>(());
                }
                test_harness::sleep_real(STEP_REAL).await;
                steps = steps.saturating_add(1);
                if steps.is_multiple_of(NUDGE_EVERY_STEPS) {
                    let _woken = self.served.clock.advance(Duration::from_secs(1));
                }
            }
        };
        test_harness::timeout_real(wait, WAIT_LIMIT)
            .await
            .map_err(|e| format!("waiting for the sweeper to flag A and B offline: {e}"))?
    }

    pub async fn settle(&self) -> Result<(), String> {
        let sql = "SELECT count(*) FROM mailbox_letters WHERE state = 'pending' AND kind = 'round'";
        let wait = async {
            let mut steps = 0u32;
            loop {
                if self.count(sql, &[]).await? == 0 {
                    return Ok::<(), String>(());
                }
                test_harness::sleep_real(STEP_REAL).await;
                steps = steps.saturating_add(1);
                if steps.is_multiple_of(NUDGE_EVERY_STEPS) {
                    let _woken = self.served.clock.advance(NUDGE_FAKE);
                }
            }
        };
        match test_harness::timeout_real(wait, WAIT_LIMIT).await {
            Ok(done) => done,
            Err(expired) => Err(format!(
                "waiting for pending letters to settle: {expired}; {}",
                self.pending_summary().await
            )),
        }
    }

    async fn pending_summary(&self) -> String {
        let rows = sqlx::query_as::<_, (String, String, i64)>(
            "SELECT device_id, state, count(*) FROM mailbox_letters WHERE state = 'pending' GROUP BY device_id, state",
        )
        .fetch_all(&self.served.pool)
        .await;
        format!(
            "{rows:?}; x log: {:?}",
            self.x
                .snapshot()
                .map(|log| (log.claimed.len(), log.refused, log.failures))
        )
    }
}
