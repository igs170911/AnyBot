use std::sync::Arc;
use std::time::Duration;

use anybot_foundation::Clock;
use serde_json::json;
use test_harness::{FakeClock, sleep_real, test_dir};

use crate::action::{ACTION_UNREACHABLE_CODE, ActionError, ActionReply};
use crate::client::SyncClient;
use crate::fake_wire::{FakeWire, problem};
use crate::random::RandomSource;
use crate::status::ConnectionState;
use crate::wire::{Wire, WireFailure};

struct Fixed;

impl RandomSource for Fixed {
    fn next_u64(&self) -> u64 {
        0
    }
}

const START_MS: u64 = 1_700_000_000_000;
const PATH: &str = "/v1/streams/actions";

async fn start(
    name: &str,
    clock: &Arc<FakeClock>,
    wire: &Arc<FakeWire>,
) -> Result<Arc<SyncClient>, String> {
    let dir = test_dir(name)?;
    let clock_dyn: Arc<dyn Clock> = Arc::<FakeClock>::clone(clock);
    let wire_dyn: Arc<dyn Wire> = Arc::<FakeWire>::clone(wire);
    let client = SyncClient::start_with_wire(&dir, wire_dyn, clock_dyn, Arc::new(Fixed))
        .await
        .map_err(|e| e.to_string())?;
    Ok(Arc::new(client))
}

type Answer = Result<ActionReply, ActionError>;

async fn run_to_end(
    client: &Arc<SyncClient>,
    clock: &Arc<FakeClock>,
    step: Duration,
) -> Result<Answer, String> {
    let asking = Arc::clone(client);
    let task = tokio::spawn(async move {
        asking
            .send_action("a1", PATH, &json!({"action": "stop"}))
            .await
    });
    for _round in 0..4000 {
        if task.is_finished() {
            return task.await.map_err(|e| e.to_string());
        }
        sleep_real(Duration::from_millis(2)).await;
        let _woken = clock.advance(step);
    }
    task.abort();
    Err("the action never ended".to_owned())
}

fn between(times: &[u64]) -> Vec<u64> {
    times
        .iter()
        .zip(times.iter().skip(1))
        .map(|(before, after)| after.saturating_sub(*before))
        .collect()
}

#[tokio::test]
async fn failed_sends_use_the_same_key_and_the_gaps_0_5_1_2_4_8_8_without_jitter()
-> Result<(), String> {
    let clock = FakeClock::new(START_MS);
    let wire = FakeWire::always_failing(&clock);
    let client = start("action_gaps", &clock, &wire).await?;
    let answer = run_to_end(&client, &clock, Duration::from_millis(50)).await?;
    assert!(matches!(answer, Err(ActionError::Unreachable)));
    let gaps = between(&wire.calls());
    let wanted = [500, 1_000, 2_000, 4_000, 8_000, 8_000, 8_000];
    for (seen, want) in gaps.iter().zip(wanted.iter()) {
        assert!((*want..*want + 100).contains(seen), "gaps: {gaps:?}");
    }
    let keys = wire.keys();
    assert!(keys.iter().all(|key| *key == Some("action-a1".to_owned())));
    assert!(keys.len() > 10, "attempts: {}", keys.len());
    Ok(())
}

#[tokio::test]
async fn waiting_attempts_end_at_20_seconds_and_the_100th_second_releases_the_one_in_flight()
-> Result<(), String> {
    let clock = FakeClock::new(START_MS);
    let wire = FakeWire::hanging(&clock);
    let client = start("action_hang", &clock, &wire).await?;
    let began = clock.now_ms();
    let answer = run_to_end(&client, &clock, Duration::from_millis(100)).await?;
    let ended = clock.now_ms().saturating_sub(began);
    assert!(matches!(answer, Err(ActionError::Unreachable)));
    assert!((100_000..100_300).contains(&ended), "ended at {ended}");
    let starts: Vec<u64> = wire
        .calls()
        .iter()
        .map(|at| at.saturating_sub(began))
        .collect();
    let wanted = [0, 20_500, 41_500, 63_500, 87_500];
    assert_eq!(starts.len(), wanted.len(), "starts: {starts:?}");
    for (seen, want) in starts.iter().zip(wanted.iter()) {
        assert!((*want..*want + 300).contains(seen), "starts: {starts:?}");
    }
    Ok(())
}

#[tokio::test]
async fn a_refusal_from_the_server_is_returned_without_retrying() -> Result<(), String> {
    let clock = FakeClock::new(START_MS);
    let wire = FakeWire::new(&clock, vec![Ok(problem(409, "record-restore-expired"))]);
    let client = start("action_refused", &clock, &wire).await?;
    let answer = run_to_end(&client, &clock, Duration::from_millis(50)).await?;
    let reply = answer.map_err(|e| e.to_string())?;
    assert_eq!(reply.status, 409);
    assert_eq!(wire.calls().len(), 1);
    Ok(())
}

#[tokio::test]
async fn a_server_error_is_retried_with_the_same_key_until_it_answers() -> Result<(), String> {
    let clock = FakeClock::new(START_MS);
    let first = Ok(problem(503, "problem-internal"));
    let second = Err(WireFailure::Network("reset".to_owned()));
    let wire = FakeWire::new(&clock, vec![first, second]);
    let client = start("action_retry", &clock, &wire).await?;
    let answer = run_to_end(&client, &clock, Duration::from_millis(50)).await?;
    let reply = answer.map_err(|e| e.to_string())?;
    assert_eq!(reply.status, 200);
    assert_eq!(wire.calls().len(), 3);
    assert_eq!(client.status().actions_retrying, 0);
    Ok(())
}

#[tokio::test]
async fn a_rejected_session_stops_the_action_and_asks_for_a_login() -> Result<(), String> {
    let clock = FakeClock::new(START_MS);
    let wire = FakeWire::new(&clock, vec![Ok(problem(401, "auth-session-invalid"))]);
    let client = start("action_login", &clock, &wire).await?;
    let answer = run_to_end(&client, &clock, Duration::from_millis(50)).await?;
    assert!(matches!(answer, Err(ActionError::NeedLogin)));
    assert_eq!(client.status().connection, ConnectionState::NeedLogin);
    assert_eq!(wire.calls().len(), 1);
    Ok(())
}

#[tokio::test]
async fn the_unreachable_error_carries_the_message_code() {
    assert_eq!(
        ActionError::Unreachable.code(),
        Some(ACTION_UNREACHABLE_CODE)
    );
}
