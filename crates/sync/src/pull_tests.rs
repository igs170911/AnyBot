use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use anybot_foundation::Clock;
use test_harness::{FakeClock, sleep_real, test_dir};

use crate::client::{StreamTarget, SyncClient};
use crate::fake_wire::{FakeWire, page_reply, problem, stored};
use crate::reconnect_random::{RandomSource, RandomUnavailable};
use crate::wire::Wire;

struct Fixed;

impl RandomSource for Fixed {
    fn next_u64(&self) -> Result<u64, RandomUnavailable> {
        Ok(0)
    }
}

const START_MS: u64 = 1_700_000_000_000;
const BOT: &str = "bot-1";

async fn start(
    name: &str,
    wire: &Arc<FakeWire>,
    clock: &Arc<FakeClock>,
) -> Result<SyncClient, String> {
    let dir = test_dir(name)?;
    let clock_dyn: Arc<dyn Clock> = Arc::<FakeClock>::clone(clock);
    let wire_dyn: Arc<dyn Wire> = Arc::<FakeWire>::clone(wire);
    SyncClient::start_with_wire(&dir, wire_dyn, clock_dyn, Arc::new(Fixed))
        .await
        .map_err(|e| e.to_string())
}

fn bot() -> StreamTarget {
    StreamTarget::Bot(BOT.to_owned())
}

async fn local_count(client: &SyncClient) -> usize {
    client
        .records(&bot())
        .await
        .map_or(usize::MAX, |rows| rows.len())
}

async fn until<F, Fut>(clock: &Arc<FakeClock>, what: &str, mut check: F) -> Result<(), String>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = bool>,
{
    for _round in 0..3000 {
        if check().await {
            return Ok(());
        }
        sleep_real(Duration::from_millis(2)).await;
        let _woken = clock.advance(Duration::from_millis(250));
    }
    Err(format!("gave up waiting for {what}"))
}

#[tokio::test]
async fn a_page_for_another_stream_is_not_applied() -> Result<(), String> {
    let clock = FakeClock::new(START_MS);
    let wire = FakeWire::new(&clock, vec![]);
    let wrong = page_reply("account", &[stored(1, "x1", "message", "a")], false, 1);
    wire.on_get("bot", vec![Ok(wrong)]);
    let client = start("sync_unit_wrong_stream", &wire, &clock).await?;
    client.pull_now(&bot());
    until(&clock, "a second ask", || async { wire.gets().len() >= 3 }).await?;
    assert_eq!(local_count(&client).await, 0);
    Ok(())
}

#[tokio::test]
async fn a_gap_in_a_page_is_asked_again_from_the_last_good_number() -> Result<(), String> {
    let clock = FakeClock::new(START_MS);
    let wire = FakeWire::new(&clock, vec![]);
    let holed = page_reply(
        BOT,
        &[
            stored(1, "x1", "message", "a"),
            stored(3, "x3", "message", "c"),
        ],
        false,
        3,
    );
    let whole = page_reply(
        BOT,
        &[
            stored(2, "x2", "message", "b"),
            stored(3, "x3", "message", "c"),
        ],
        false,
        3,
    );
    wire.on_get("bot", vec![Ok(holed), Ok(whole)]);
    let client = start("sync_unit_gap", &wire, &clock).await?;
    client.pull_now(&bot());
    until(&clock, "three records", || async {
        local_count(&client).await == 3
    })
    .await?;
    let asks: Vec<String> = wire
        .gets()
        .into_iter()
        .filter(|p| p.contains("bot"))
        .collect();
    assert!(
        asks.iter().any(|path| path.contains("after=1")),
        "asks: {asks:?}"
    );
    Ok(())
}

#[tokio::test]
async fn a_page_that_stops_before_the_servers_last_number_is_asked_again() -> Result<(), String> {
    let clock = FakeClock::new(START_MS);
    let wire = FakeWire::new(&clock, vec![]);
    let short = page_reply(
        BOT,
        &[
            stored(1, "x1", "message", "a"),
            stored(2, "x2", "message", "b"),
        ],
        false,
        3,
    );
    let rest = page_reply(BOT, &[stored(3, "x3", "message", "c")], false, 3);
    wire.on_get("bot", vec![Ok(short), Ok(rest)]);
    let client = start("sync_unit_short_page", &wire, &clock).await?;
    client.pull_now(&bot());
    until(&clock, "three records", || async {
        local_count(&client).await == 3
    })
    .await?;
    Ok(())
}

#[tokio::test]
async fn resending_after_a_mismatch_skips_server_written_kinds_and_counts_them()
-> Result<(), String> {
    let clock = FakeClock::new(START_MS);
    let wire = FakeWire::new(&clock, vec![]);
    let had = page_reply(
        BOT,
        &[
            stored(1, "x1", "message", "a"),
            stored(2, "x2", "referee_stop", "stop"),
            stored(3, "x3", "message", "c"),
        ],
        false,
        3,
    );
    let restored = page_reply(BOT, &[stored(1, "x1", "message", "a")], false, 1);
    wire.on_get(
        "bot",
        vec![
            Ok(had),
            Err(crate::wire::WireFailure::Network("later".to_owned())),
            Ok(problem(409, "sync-after-ahead")),
            Ok(restored),
        ],
    );
    let client = start("sync_unit_resend_counts", &wire, &clock).await?;
    client.pull_now(&bot());
    until(&clock, "three records", || async {
        local_count(&client).await == 3
    })
    .await?;
    client.pull_now(&bot());
    until(&clock, "a resync notice", || async {
        !client.status().resyncs.is_empty()
    })
    .await?;
    let notice = client.status().resyncs.last().cloned().ok_or("no notice")?;
    assert_eq!((notice.resent, notice.server_written_not_resent), (1, 1));
    assert_eq!(client.status().unrecovered_kinds, 1);
    until(&clock, "the resend", || async { !wire.posted().is_empty() }).await?;
    let sent = wire.posted();
    let ids: Vec<String> = sent
        .iter()
        .filter_map(|body| body.get("items").and_then(|items| items.as_array()))
        .flatten()
        .filter_map(|item| {
            item.get("device_id")
                .and_then(|id| id.as_str())
                .map(str::to_owned)
        })
        .collect();
    assert_eq!(ids, ["x3"]);
    Ok(())
}
