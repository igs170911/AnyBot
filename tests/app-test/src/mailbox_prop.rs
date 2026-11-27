use std::time::Duration;

use serde_json::{Value, json};

use crate::http::{ConnectionPool, RequestSpec, ResponseSpec};
use crate::mailbox_model::{Dev, LONG_PASS_SECS, Op, Outcome, Plan};
use crate::mailbox_x::XStandIn;
use crate::serve::Served;

const STEP_REAL: Duration = Duration::from_millis(20);
const NUDGE_EVERY_STEPS: u32 = 8;
const NUDGE_FAKE: Duration = Duration::from_secs(5);
const WAIT_LIMIT: Duration = Duration::from_secs(30);

#[derive(Clone)]
pub struct Seat {
    pub bearer: String,
    pub device_id: String,
}

pub struct Seats {
    pub phone: String,
    pub a: Seat,
    pub b: Seat,
    pub x: Seat,
    pub bot: String,
}

pub struct Rig<'a> {
    pub served: &'a Served,
    pub seats: Seats,
    pub ids: Vec<String>,
    pub x: XStandIn,
    base: String,
    http: ConnectionPool,
    keys: u64,
}

pub async fn make_bot(served: &Served, phone: &str, bot_id: &str, key: &str) -> Result<(), String> {
    let item = json!({
        "device_id": "dev-setup",
        "kind": "bots_create",
        "content": {"bot_id": bot_id, "name": "helper"},
        "created_at_ms": 1_700_000_000_100_i64,
        "offline_sent": false,
    });
    let body = json!({"stream": "account", "items": [item]}).to_string();
    let spec = RequestSpec::new(&body).with_key(key).with_bearer(phone);
    let landed = crate::http::post(&served.base_url(), "/v1/streams/append", &spec).await?;
    if landed.status == 200 {
        return Ok(());
    }
    Err(format!(
        "bot create refused: {} {}",
        landed.status, landed.json
    ))
}

impl<'a> Rig<'a> {
    pub async fn open(served: &'a Served, seats: Seats) -> Result<Self, String> {
        let base = served.base_url();
        let x = XStandIn::start(&base, &seats.x.bearer).await?;
        Ok(Rig {
            served,
            seats,
            ids: Vec::new(),
            x,
            base,
            http: ConnectionPool::new(16)?,
            keys: 0,
        })
    }

    fn seat(&self, dev: Dev) -> &Seat {
        match dev {
            Dev::A => &self.seats.a,
            Dev::B => &self.seats.b,
            Dev::X => &self.seats.x,
        }
    }

    async fn call(
        &mut self,
        bearer: &str,
        path: &str,
        body: &Value,
    ) -> Result<ResponseSpec, String> {
        self.keys = self.keys.saturating_add(1);
        let key = format!("prop-{}", self.keys);
        let spec = RequestSpec::new(&body.to_string())
            .with_key(&key)
            .with_bearer(bearer);
        self.http.post(&self.base, path, &spec).await
    }

    pub async fn run(&mut self, plan: &Plan) -> Result<(), String> {
        for (number, op) in plan.ops.iter().enumerate() {
            self.exec(op)
                .await
                .map_err(|reason| format!("op {number} {op:?}: {reason}"))?;
        }
        Ok(())
    }

    async fn exec(&mut self, op: &Op) -> Result<(), String> {
        match op {
            Op::Deliver {
                slot,
                named,
                lands_on,
                reassigned,
            } => self.deliver(*slot, *named, *lands_on, *reassigned).await,
            Op::Claim { slot, by, outcome } => self.claim_slot(*slot, *by, *outcome).await,
            Op::Race { slot, by } => self.race(*slot, *by).await,
            Op::XAgain { pick } => self.x_again(*pick).await,
            Op::XStranger { by, pick } => self.x_stranger(*by, *pick).await,
            Op::Pass { secs } => self.pass(*secs).await,
            Op::Reconnect { dev } => self.reconnect(*dev).await,
        }
    }

    async fn deliver(
        &mut self,
        slot: usize,
        named: Option<Dev>,
        lands_on: Dev,
        reassigned: bool,
    ) -> Result<(), String> {
        let computer = named.map(|dev| self.seat(dev).device_id.clone());
        let body = json!({"bot": self.seats.bot, "computer": computer});
        let phone = self.seats.phone.clone();
        let answer = self.call(&phone, "/v1/mailbox/round", &body).await?;
        if answer.status != 200 {
            return Err(format!("deliver refused: {}", answer.json));
        }
        let landed = answer.field_str("device_id")?;
        let expected = self.seat(lands_on).device_id.clone();
        if landed != expected {
            return Err(format!("landed on {landed}, wanted {expected}"));
        }
        if answer.json.get("reassigned").is_some() != reassigned {
            return Err(format!("reassigned flag differs: {}", answer.json));
        }
        if self.ids.len() != slot {
            return Err("slots out of order".to_owned());
        }
        self.ids.push(answer.field_str("letter_id")?);
        Ok(())
    }

    async fn claim_id(&mut self, letter: &str, by: Dev, outcome: Outcome) -> Result<(), String> {
        let bearer = self.seat(by).bearer.clone();
        let answer = self
            .call(&bearer, "/v1/mailbox/claim", &json!({"letter_id": letter}))
            .await?;
        match_outcome(&answer, outcome)
    }

    fn spec_for(&mut self, bearer: &str, body: &Value) -> RequestSpec {
        self.keys = self.keys.saturating_add(1);
        let key = format!("prop-{}", self.keys);
        RequestSpec::new(&body.to_string())
            .with_key(&key)
            .with_bearer(bearer)
    }

    async fn race(&mut self, slot: usize, by: Dev) -> Result<(), String> {
        let letter = self.ids.get(slot).cloned().ok_or("slot has no letter")?;
        let bearer = self.seat(by).bearer.clone();
        let body = json!({"letter_id": letter});
        let first = self.spec_for(&bearer, &body);
        let second = self.spec_for(&bearer, &body);
        let path = "/v1/mailbox/claim";
        let (one, two) = tokio::join!(
            self.http.post(&self.base, path, &first),
            self.http.post(&self.base, path, &second)
        );
        let (one, two) = (one?, two?);
        let (winner, loser) = if one.status == 200 {
            (one, two)
        } else {
            (two, one)
        };
        match_outcome(&winner, Outcome::Won)?;
        match_outcome(&loser, Outcome::Refused(crate::mailbox_model::CODE_YOURS))
    }

    async fn claim_slot(&mut self, slot: usize, by: Dev, outcome: Outcome) -> Result<(), String> {
        let letter = self.ids.get(slot).cloned().ok_or("slot has no letter")?;
        self.claim_id(&letter, by, outcome).await
    }

    async fn x_letter(&self, pick: u64) -> Result<String, String> {
        let wait = async {
            loop {
                if let Some(letter) = self.x.claimed_at(pick)? {
                    return Ok::<String, String>(letter);
                }
                test_harness::sleep_real(STEP_REAL).await;
            }
        };
        test_harness::timeout_real(wait, WAIT_LIMIT).await?
    }

    async fn x_again(&mut self, pick: u64) -> Result<(), String> {
        let letter = self.x_letter(pick).await?;
        self.claim_id(
            &letter,
            Dev::X,
            Outcome::Refused(crate::mailbox_model::CODE_YOURS),
        )
        .await
    }

    async fn x_stranger(&mut self, by: Dev, pick: u64) -> Result<(), String> {
        let letter = self.x_letter(pick).await?;
        self.claim_id(
            &letter,
            by,
            Outcome::Refused(crate::mailbox_model::CODE_TAKEN),
        )
        .await
    }

    pub async fn reconnect_all(&mut self) -> Result<(), String> {
        self.reconnect(Dev::A).await?;
        self.reconnect(Dev::B).await
    }

    async fn reconnect(&mut self, dev: Dev) -> Result<(), String> {
        let bearer = self.seat(dev).bearer.clone();
        let spec = RequestSpec::new("").with_bearer(&bearer);
        let answer = self.http.get(&self.base, "/v1/computers", &spec).await?;
        if answer.status == 200 {
            return Ok(());
        }
        Err(format!("reconnect refused: {}", answer.json))
    }

    async fn pass(&mut self, secs: u64) -> Result<(), String> {
        let _woken = self.served.clock.advance(Duration::from_secs(secs));
        if secs >= LONG_PASS_SECS {
            self.wait_until_flagged_offline().await?;
        }
        Ok(())
    }

    async fn count(&self, sql: &'static str, binds: &[&str]) -> Result<i64, String> {
        let mut query = sqlx::query_scalar::<_, i64>(sql);
        for bind in binds {
            query = query.bind(*bind);
        }
        query
            .fetch_one(&self.served.pool)
            .await
            .map_err(|e| e.to_string())
    }

    async fn wait_until_flagged_offline(&self) -> Result<(), String> {
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

fn match_outcome(answer: &ResponseSpec, outcome: Outcome) -> Result<(), String> {
    let (status, code) = match outcome {
        Outcome::Won => (200, None),
        Outcome::Refused(code) => (409, Some(code)),
        Outcome::Unknown => (404, Some("mailbox-letter-unknown")),
    };
    if answer.status != status {
        return Err(format!(
            "wanted status {status}, got {} {}",
            answer.status, answer.json
        ));
    }
    if answer.code().as_deref() != code {
        return Err(format!("wanted code {code:?}, got {}", answer.json));
    }
    Ok(())
}
