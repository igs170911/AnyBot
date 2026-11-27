use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use anybot_foundation::Clock;
use test_harness::{FakeClock, sleep_real, test_dir};

use crate::client::{StreamTarget, SyncClient};
use crate::fake_wire::FakeWire;
use crate::link::Link;
use crate::link_task::run_keepalive;
use crate::reconnect_random::{RandomSource, RandomUnavailable};
use crate::reconnect_status::LinkPhase;
use crate::wire::Wire;

struct Fixed;

impl RandomSource for Fixed {
    fn next_u64(&self) -> Result<u64, RandomUnavailable> {
        Ok(0)
    }
}

const START_MS: u64 = 1_700_000_000_000;

fn link_on(clock: &Arc<FakeClock>) -> Link {
    let shared: Arc<dyn Clock> = Arc::<FakeClock>::clone(clock);
    Link::new(&shared, Arc::new(Fixed))
}

async fn client_on(
    name: &str,
    clock: &Arc<FakeClock>,
) -> Result<(SyncClient, Arc<FakeWire>), String> {
    let wire = FakeWire::new(clock, vec![]);
    let dir = test_dir(name)?;
    let clock_dyn: Arc<dyn Clock> = Arc::<FakeClock>::clone(clock);
    let wire_dyn: Arc<dyn Wire> = Arc::<FakeWire>::clone(&wire);
    let client = SyncClient::start_with_wire(&dir, wire_dyn, clock_dyn, Arc::new(Fixed))
        .await
        .map_err(|e| e.to_string())?;
    Ok((client, wire))
}

fn account_asks(wire: &FakeWire) -> usize {
    wire.gets()
        .iter()
        .filter(|path| path.contains("stream=account"))
        .count()
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

async fn walk_seconds<F>(clock: &Arc<FakeClock>, seconds: u64, mut each: F)
where
    F: FnMut(u64),
{
    for second in 1..=seconds {
        let _woken = clock.advance(Duration::from_secs(1));
        each(second);
        sleep_real(Duration::from_millis(5)).await;
    }
    sleep_real(Duration::from_millis(50)).await;
}

#[test]
fn a_second_failure_inside_the_same_wait_is_not_counted_again() -> Result<(), String> {
    let clock = FakeClock::new(START_MS);
    let link = link_on(&clock);
    link.failed().map_err(|e| e.reason)?;
    link.failed().map_err(|e| e.reason)?;
    assert_eq!(link.pacer.failures_in_row(), 1);
    assert!(link.waiting());
    let _woken = clock.advance(Duration::from_secs(3));
    assert!(!link.waiting());
    link.failed().map_err(|e| e.reason)?;
    assert_eq!(link.pacer.failures_in_row(), 2);
    Ok(())
}

#[tokio::test]
async fn keepalive_asks_for_the_account_stream_every_20_seconds_while_sse_is_live()
-> Result<(), String> {
    let clock = FakeClock::new(START_MS);
    let (client, wire) = client_on("sync_unit_keepalive_live", &clock).await?;
    real_until("the first account ask", || async {
        account_asks(&wire) == 1
    })
    .await?;
    client.core.link.set_sse_live(true);
    let keepalive = tokio::spawn(run_keepalive(Arc::clone(&client.core)));
    walk_seconds(&clock, 120, |_second| {}).await;
    keepalive.abort();
    let asked = account_asks(&wire).saturating_sub(1);
    assert!((5..=6).contains(&asked), "{asked} keepalive asks in 120 s");
    Ok(())
}

#[tokio::test]
async fn keepalive_stays_quiet_while_other_requests_keep_the_link_busy() -> Result<(), String> {
    let clock = FakeClock::new(START_MS);
    let (client, wire) = client_on("sync_unit_keepalive_busy", &clock).await?;
    real_until("the first account ask", || async {
        account_asks(&wire) == 1
    })
    .await?;
    client.core.link.set_sse_live(true);
    let keepalive = tokio::spawn(run_keepalive(Arc::clone(&client.core)));
    let bot = StreamTarget::Bot("bot-1".to_owned());
    walk_seconds(&clock, 120, |second| {
        if second.checked_rem(10) == Some(0) {
            client.pull_now(&bot);
        }
    })
    .await;
    keepalive.abort();
    assert_eq!(account_asks(&wire), 1);
    Ok(())
}

#[tokio::test]
async fn keepalive_sends_nothing_while_sse_is_not_connected() -> Result<(), String> {
    let clock = FakeClock::new(START_MS);
    let (client, wire) = client_on("sync_unit_keepalive_down", &clock).await?;
    real_until("the first account ask", || async {
        account_asks(&wire) == 1
    })
    .await?;
    let keepalive = tokio::spawn(run_keepalive(Arc::clone(&client.core)));
    walk_seconds(&clock, 120, |_second| {}).await;
    keepalive.abort();
    assert_eq!(account_asks(&wire), 1);
    Ok(())
}

#[tokio::test]
async fn dropping_the_client_stops_its_background_tasks() -> Result<(), String> {
    let clock = FakeClock::new(START_MS);
    let (client, _wire) = client_on("sync_unit_drop_stops", &clock).await?;
    let core = Arc::clone(&client.core);
    assert!(Arc::strong_count(&core) > 2);
    drop(client);
    real_until("every task let go of the core", || async {
        Arc::strong_count(&core) == 1
    })
    .await
}

#[tokio::test]
async fn link_status_from_the_board_shows_up_in_the_client_status() -> Result<(), String> {
    let clock = FakeClock::new(START_MS);
    let (client, _wire) = client_on("sync_unit_link_mirror", &clock).await?;
    real_until("the connecting phase in the status", || async {
        client.status().link.map(|link| link.phase) == Some(LinkPhase::Connecting)
    })
    .await?;
    client.core.link.board.connected();
    real_until("the connected phase in the status", || async {
        client.status().link.map(|link| link.phase) == Some(LinkPhase::Connected)
    })
    .await
}
