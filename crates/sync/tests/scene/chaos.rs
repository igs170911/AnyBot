use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};

use anybot_protocol::{AfterPage, StoredRecord};
use anybot_sync::{Wire, WireFailure, WireFuture, WireMethod, WireReply, WireRequest};

const SEEN_PAGES_KEPT: usize = 40;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Tally {
    pub(crate) dropped_replies: u64,
    pub(crate) stale_replays: u64,
    pub(crate) dropped_records: u64,
    pub(crate) duplicated_records: u64,
    pub(crate) shuffled_records: u64,
}

impl Tally {
    pub(crate) fn events(&self) -> u64 {
        self.dropped_replies
            .saturating_add(self.stale_replays)
            .saturating_add(self.dropped_records)
            .saturating_add(self.duplicated_records)
            .saturating_add(self.shuffled_records)
    }
}

struct State {
    rng: u64,
    tally: Tally,
    seen: HashMap<String, Vec<Vec<u8>>>,
}

impl State {
    fn next(&mut self) -> u64 {
        let mut x = self.rng;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.rng = x;
        x
    }

    fn chance(&mut self, percent: u64) -> bool {
        self.next()
            .checked_rem(100)
            .is_some_and(|roll| roll < percent)
    }

    fn below(&mut self, bound: usize) -> usize {
        let bound = u64::try_from(bound).unwrap_or(1).max(1);
        let pick = self.next().checked_rem(bound).unwrap_or(0);
        usize::try_from(pick).unwrap_or(0)
    }
}

pub(crate) struct Chaos {
    inner: Arc<dyn Wire>,
    state: Mutex<State>,
}

impl Chaos {
    pub(crate) fn new(inner: Arc<dyn Wire>, seed: u64) -> Arc<Chaos> {
        Arc::new(Chaos {
            inner,
            state: Mutex::new(State {
                rng: seed.max(1),
                tally: Tally::default(),
                seen: HashMap::new(),
            }),
        })
    }

    pub(crate) fn tally(&self) -> Tally {
        self.lock().tally.clone()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn disturb(&self, reply: WireReply) -> Result<WireReply, WireFailure> {
        let Ok(mut page) = serde_json::from_slice::<AfterPage>(&reply.body) else {
            return Ok(reply);
        };
        let mut state = self.lock();
        if state.chance(7) {
            state.tally.dropped_replies = state.tally.dropped_replies.saturating_add(1);
            return Err(WireFailure::Network("chaos: reply dropped".to_owned()));
        }
        let key = format!("{:?}", page.stream);
        let history_len = state.seen.get(&key).map_or(0, Vec::len);
        if history_len > 0 && state.chance(5) {
            let pick = state.below(history_len);
            state.tally.stale_replays = state.tally.stale_replays.saturating_add(1);
            let body = state
                .seen
                .get(&key)
                .and_then(|history| history.get(pick))
                .cloned()
                .unwrap_or_default();
            return Ok(WireReply { body, ..reply });
        }
        let good = reply.body.clone();
        page.records = scramble(&mut state, page.records);
        let history = state.seen.entry(key).or_default();
        history.push(good);
        if history.len() > SEEN_PAGES_KEPT {
            history.remove(0);
        }
        let body = serde_json::to_vec(&page).map_err(|e| WireFailure::Network(e.to_string()))?;
        Ok(WireReply { body, ..reply })
    }
}

fn scramble(state: &mut State, records: Vec<StoredRecord>) -> Vec<StoredRecord> {
    let mut kept = Vec::with_capacity(records.len());
    for record in records {
        if state.chance(12) {
            state.tally.dropped_records = state.tally.dropped_records.saturating_add(1);
            continue;
        }
        if state.chance(12) {
            state.tally.duplicated_records = state.tally.duplicated_records.saturating_add(1);
            kept.push(record.clone());
        }
        kept.push(record);
    }
    if state.chance(35) {
        let size = kept.len();
        for index in (1..size).rev() {
            let other = state.below(index.saturating_add(1));
            kept.swap(index, other);
        }
        state.tally.shuffled_records = state
            .tally
            .shuffled_records
            .saturating_add(u64::try_from(size).unwrap_or(0));
    }
    kept
}

impl Wire for Chaos {
    fn call(&self, request: WireRequest) -> WireFuture<'_> {
        Box::pin(async move {
            let after = request.method == WireMethod::Get;
            let reply = self.inner.call(request).await?;
            if after {
                self.disturb(reply)
            } else {
                Ok(reply)
            }
        })
    }
}
