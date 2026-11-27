use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use anybot_foundation::Clock;
use serde_json::json;
use test_harness::{FakeClock, sleep_real, test_dir};

use crate::client::{NewRecord, StreamTarget, SyncClient};
use crate::fake_wire::{FakeWire, problem};
use crate::random::RandomSource;
use crate::status::ConnectionState;
use crate::wire::{Wire, WireFailure};

struct Fixed(u64);

impl RandomSource for Fixed {
    fn next_u64(&self) -> u64 {
        self.0
    }
}

const START_MS: u64 = 1_700_000_000_000;
const STEP: Duration = Duration::from_millis(50);

fn chat(text: &str) -> NewRecord {
    NewRecord {
        stream: StreamTarget::Account,
        kind: "message".to_owned(),
        content: json!({"text": text}),
        task_id: None,
    }
}

async fn start(
    name: &str,
    clock: &Arc<FakeClock>,
    wire: &Arc<FakeWire>,
) -> Result<SyncClient, String> {
    let dir = test_dir(name)?;
    let clock_dyn: Arc<dyn Clock> = Arc::<FakeClock>::clone(clock);
    let wire_dyn: Arc<dyn Wire> = Arc::<FakeWire>::clone(wire);
    SyncClient::start_with_wire(&dir, wire_dyn, clock_dyn, Arc::new(Fixed(0)))
        .await
        .map_err(|e| e.to_string())
}

async fn real_until<F, Fut>(what: &str, mut check: F) -> Result<(), String>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = bool>,
{
    for _round in 0..500 {
        if check().await {
            return Ok(());
        }
        sleep_real(Duration::from_millis(10)).await;
    }
    Err(format!("gave up waiting for {what}"))
}

async fn drive_until<F, Fut>(what: &str, clock: &Arc<FakeClock>, mut check: F) -> Result<(), String>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = bool>,
{
    for _round in 0..4000 {
        if check().await {
            return Ok(());
        }
        sleep_real(Duration::from_millis(2)).await;
        let _woken = clock.advance(STEP);
    }
    Err(format!("gave up driving the clock until {what}"))
}

fn unreachable(client: &SyncClient) -> bool {
    matches!(
        client.status().connection,
        ConnectionState::Unreachable { .. }
    )
}

#[tokio::test]
async fn sends_keep_at_least_two_hundred_milliseconds_apart() -> Result<(), String> {
    let clock = FakeClock::new(START_MS);
    let wire = FakeWire::new(&clock, vec![]);
    let client = start("sync_unit_pace", &clock, &wire).await?;
    client
        .add_record(chat("1"))
        .await
        .map_err(|e| e.to_string())?;
    real_until("first send", || async { wire.calls().len() == 1 }).await?;
    client
        .add_record(chat("2"))
        .await
        .map_err(|e| e.to_string())?;
    drive_until("second send", &clock, || async { wire.calls().len() == 2 }).await?;
    assert_eq!(wire.gaps().len(), 1);
    for gap in wire.gaps() {
        assert!((200..450).contains(&gap), "gap was {gap} ms");
    }
    Ok(())
}

#[tokio::test]
async fn retry_gaps_follow_the_schedule_plus_the_random_part() -> Result<(), String> {
    let clock = FakeClock::new(START_MS);
    let script = (0..7)
        .map(|_| Err(WireFailure::Network("down".to_owned())))
        .collect();
    let wire = FakeWire::new(&clock, script);
    let client = start("sync_unit_retry", &clock, &wire).await?;
    client
        .add_record(chat("1"))
        .await
        .map_err(|e| e.to_string())?;
    drive_until("eighth call", &clock, || async { wire.calls().len() >= 8 }).await?;
    let expected = [1_500, 2_500, 4_500, 8_500, 16_500, 30_500, 30_500];
    let gaps = wire.gaps();
    assert_eq!(gaps.len(), expected.len());
    for (gap, want) in gaps.iter().zip(expected) {
        assert!((want..want + 1_000).contains(gap), "gap {gap} want {want}");
    }
    real_until("outbox empty", || async {
        client.outbox().await.is_ok_and(|rows| rows.is_empty())
    })
    .await?;
    assert_eq!(client.status().rejected, 0);
    Ok(())
}

#[tokio::test]
async fn a_refused_connection_keeps_the_record_cancellable() -> Result<(), String> {
    let clock = FakeClock::new(START_MS);
    let failure = Err(WireFailure::ConnectFailed("refused".to_owned()));
    let wire = FakeWire::new(&clock, vec![failure]);
    let client = start("sync_unit_connect_failed", &clock, &wire).await?;
    let id = client
        .add_record(chat("1"))
        .await
        .map_err(|e| e.to_string())?;
    real_until("unreachable", || async { unreachable(&client) }).await?;
    assert!(client.can_cancel(&id).await.map_err(|e| e.to_string())?);
    let taken = client.cancel_unsent(&id).await.map_err(|e| e.to_string())?;
    assert_eq!(taken.map(|row| row.record_id), Some(id));
    Ok(())
}

#[tokio::test]
async fn a_lost_reply_makes_the_record_uncancellable() -> Result<(), String> {
    let clock = FakeClock::new(START_MS);
    let wire = FakeWire::new(&clock, vec![Err(WireFailure::WaitTimeout)]);
    let client = start("sync_unit_lost_reply", &clock, &wire).await?;
    let id = client
        .add_record(chat("1"))
        .await
        .map_err(|e| e.to_string())?;
    real_until("unreachable", || async { unreachable(&client) }).await?;
    assert!(!client.can_cancel(&id).await.map_err(|e| e.to_string())?);
    let taken = client.cancel_unsent(&id).await.map_err(|e| e.to_string())?;
    assert!(taken.is_none());
    Ok(())
}

#[tokio::test]
async fn a_record_cancelled_while_waiting_for_the_pace_is_never_sent() -> Result<(), String> {
    let clock = FakeClock::new(START_MS);
    let wire = FakeWire::new(&clock, vec![]);
    let client = start("sync_unit_cancel_pace", &clock, &wire).await?;
    client
        .add_record(chat("1"))
        .await
        .map_err(|e| e.to_string())?;
    real_until("first send", || async { wire.calls().len() == 1 }).await?;
    let second = client
        .add_record(chat("2"))
        .await
        .map_err(|e| e.to_string())?;
    sleep_real(Duration::from_millis(100)).await;
    let taken = client
        .cancel_unsent(&second)
        .await
        .map_err(|e| e.to_string())?;
    assert!(taken.is_some());
    let _woken = clock.advance(Duration::from_secs(1));
    sleep_real(Duration::from_millis(200)).await;
    assert_eq!(wire.calls().len(), 1);
    Ok(())
}

#[tokio::test]
async fn a_whole_request_refusal_marks_the_first_record_and_the_rest_go() -> Result<(), String> {
    let clock = FakeClock::new(START_MS);
    let refusal = Ok(problem(403, "stream-bot-missing"));
    let wire = FakeWire::new(&clock, vec![refusal]);
    let client = start("sync_unit_refusal", &clock, &wire).await?;
    client
        .add_record(chat("1"))
        .await
        .map_err(|e| e.to_string())?;
    client
        .add_record(chat("2"))
        .await
        .map_err(|e| e.to_string())?;
    drive_until("second call", &clock, || async { wire.calls().len() >= 2 }).await?;
    real_until("only the refused record is left", || async {
        client.outbox().await.is_ok_and(|rows| rows.len() == 1)
    })
    .await?;
    let rows = client.outbox().await.map_err(|e| e.to_string())?;
    assert_eq!(
        rows.first()
            .and_then(|row| row.reject_code.clone())
            .as_deref(),
        Some("stream-bot-missing")
    );
    Ok(())
}

#[tokio::test]
async fn dropping_the_client_stops_every_background_request() -> Result<(), String> {
    let clock = FakeClock::new(START_MS);
    let wire = FakeWire::always_failing(&clock);
    let client = start("sync_unit_drop_stops", &clock, &wire).await?;
    client
        .add_record(chat("1"))
        .await
        .map_err(|e| e.to_string())?;
    drive_until("three calls", &clock, || async { wire.calls().len() >= 3 }).await?;
    drop(client);
    sleep_real(Duration::from_millis(50)).await;
    let before = wire.calls().len();
    for _round in 0..400 {
        let _woken = clock.advance(Duration::from_secs(1));
        sleep_real(Duration::from_millis(2)).await;
    }
    assert_eq!(wire.calls().len(), before);
    Ok(())
}
