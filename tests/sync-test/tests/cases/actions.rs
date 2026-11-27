use std::future::Future;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use anybot_foundation::{Clock, Secret, TrustedCerts};
use anybot_sync::http_wire::HttpWire;
use anybot_sync::reconnect_status::RequestGuard;
use anybot_sync::{
    ActionError, ActionReply, NewRecord, SyncClient, Wire, WireFailure, WireFuture, WireRequest,
};
use serde_json::{Value, json};
use test_harness::netctl::{net_drop_response, net_hang_connect, net_restore};
use test_harness::{FakeClock, sleep_real};

use crate::common::{CLIENT_VERSION, Counting, Rig, drive_until, message, rig};
use crate::scene::tap::Tap;
use crate::scene::{BOT, BOT_STREAM_ID, bot, create_bot, same_as_server};

const STEP: Duration = Duration::from_millis(250);
const FINE_STEP: Duration = Duration::from_millis(50);
const ACTIONS: &str = "/v1/streams/actions";

fn clear_account() -> Value {
    json!({"stream": "account", "action": "clear"})
}

type Answer = Result<ActionReply, ActionError>;

async fn ask(
    client: &Arc<SyncClient>,
    rig: &Rig,
    action_id: &str,
    body: Value,
) -> Result<Answer, String> {
    ask_stepping(client, rig, action_id, body, STEP).await
}

async fn ask_stepping(
    client: &Arc<SyncClient>,
    rig: &Rig,
    action_id: &str,
    body: Value,
    step: Duration,
) -> Result<Answer, String> {
    let asking = Arc::clone(client);
    let id = action_id.to_owned();
    let task = tokio::spawn(async move { asking.send_action(&id, ACTIONS, &body).await });
    for _round in 0..6000 {
        if task.is_finished() {
            return task.await.map_err(|e| e.to_string());
        }
        sleep_real(Duration::from_millis(10)).await;
        let _woken = rig.clock().advance(step);
    }
    task.abort();
    Err("the action never ended".to_owned())
}

fn proxy_addr(rig: &Rig) -> SocketAddr {
    SocketAddr::from(([127, 0, 0, 1], rig.proxy_port))
}

#[tokio::test]
async fn t_account_n6_5() -> Result<(), String> {
    let rig = rig("sync_account_n6_5").await?;
    let dir = test_harness::test_dir("sync_account_n6_5_client")?;
    let tap = Tap::start_timed(proxy_addr(&rig), Some(rig.clock())).await?;
    net_drop_response(&rig.ctl, rig.proxy_port, false).await?;
    let client = Arc::new(rig.client_at(&tap.url(), &dir, Counting::arc(0)).await?);
    let began = rig.clock().now_ms();
    let answer = ask_stepping(&client, &rig, "stop-1", clear_account(), FINE_STEP).await?;
    let ended = rig.clock().now_ms().saturating_sub(began);
    let error = answer.err().ok_or("the action should not have succeeded")?;
    assert_eq!(error.code(), Some("account-action-unreachable"));
    assert!((100_000..100_150).contains(&ended), "ended at {ended}");
    let sent: Vec<_> = tap
        .seen()
        .into_iter()
        .filter(|seen| seen.request.contains(ACTIONS))
        .collect();
    let starts: Vec<u64> = sent
        .iter()
        .filter_map(|seen| seen.at_ms)
        .map(|at| at.saturating_sub(began))
        .collect();
    let wanted = [0, 20_500, 41_500, 63_500, 87_500];
    assert_eq!(starts.len(), wanted.len(), "starts: {starts:?}");
    for (seen, want) in starts.iter().zip(wanted) {
        assert!((want..want + 150).contains(seen), "starts: {starts:?}");
    }
    let keys: Vec<_> = sent.iter().map(|seen| seen.key.clone()).collect();
    let first_key = keys.first().cloned().flatten();
    assert!(first_key.is_some());
    assert!(keys.iter().all(|key| *key == first_key));
    drive_until("quiet after the release", &rig.clock(), STEP, || async {
        Ok(rig.clock().now_ms() > began + 130_000)
    })
    .await?;
    let later = tap
        .seen()
        .into_iter()
        .filter(|seen| seen.request.contains(ACTIONS))
        .count();
    assert_eq!(later, wanted.len());
    net_restore(&rig.ctl, rig.proxy_port).await?;
    drop(client);
    rig.served.stop().await
}

struct Seen {
    path: String,
    key: Option<String>,
    started: u64,
    ended: u64,
    failure: Option<WireFailure>,
}

struct Recording {
    inner: HttpWire,
    clock: Arc<FakeClock>,
    seen: Mutex<Vec<Seen>>,
}

impl Wire for Recording {
    fn call<'a>(&'a self, request: WireRequest, guard: &'a RequestGuard) -> WireFuture<'a> {
        Box::pin(async move {
            let started = self.clock.now_ms();
            let path = request.path.clone();
            let key = request.idempotency_key.clone();
            let answer = self.inner.call(request, guard).await;
            let ended = self.clock.now_ms();
            let failure = answer.as_ref().err().cloned();
            self.seen
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(Seen {
                    path,
                    key,
                    started,
                    ended,
                    failure,
                });
            answer
        })
    }
}

#[tokio::test]
async fn t_conn_post_4() -> Result<(), String> {
    let rig = rig("sync_conn_post_4").await?;
    let dir = test_harness::test_dir("sync_conn_post_4_client")?;
    net_hang_connect(&rig.ctl, rig.proxy_port).await?;
    let shared: Arc<dyn Clock> = rig.clock_dyn();
    let proxy = format!("http://127.0.0.1:{}", rig.proxy_port);
    let inner = HttpWire::new(
        "https://203.0.113.5:8443",
        &Secret::from_untrusted(rig.bearer.clone()),
        CLIENT_VERSION,
        &TrustedCerts::empty(),
        Some(&proxy),
        Arc::clone(&shared),
    )?;
    let recording = Arc::new(Recording {
        inner,
        clock: rig.clock(),
        seen: Mutex::new(Vec::new()),
    });
    let wire: Arc<dyn Wire> = Arc::<Recording>::clone(&recording);
    let client = SyncClient::start_with_wire(&dir, wire, shared, Counting::arc(0))
        .await
        .map_err(|e| e.to_string())?;
    let client = Arc::new(client);
    let asking = Arc::clone(&client);
    let task = tokio::spawn(async move {
        asking
            .send_action("stop-2", ACTIONS, &clear_account())
            .await
    });
    drive_until("a second try", &rig.clock(), STEP, || async {
        Ok(action_tries(&recording) >= 2)
    })
    .await?;
    task.abort();
    let tries = action_rows(&recording);
    let first = tries.first().ok_or("no first try")?;
    let second = tries.get(1).ok_or("no second try")?;
    let took = first.ended.saturating_sub(first.started);
    assert!((5_000..5_400).contains(&took), "first try took {took} ms");
    assert_eq!(first.failure, Some(WireFailure::ConnectTimeout));
    assert!(second.started >= first.ended);
    assert_eq!(first.key, second.key);
    assert!(first.key.is_some());
    drop(client);
    rig.served.stop().await
}

fn action_tries(recording: &Recording) -> usize {
    action_rows(recording).len()
}

fn action_rows(recording: &Recording) -> Vec<Seen> {
    recording
        .seen
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .iter()
        .filter(|seen| seen.path == ACTIONS)
        .map(|seen| Seen {
            path: seen.path.clone(),
            key: seen.key.clone(),
            started: seen.started,
            ended: seen.ended,
            failure: seen.failure.clone(),
        })
        .collect()
}

async fn until<F, Fut>(rig: &Rig, what: &str, check: F) -> Result<(), String>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<bool, String>>,
{
    drive_until(what, &rig.clock(), STEP, check).await
}

#[tokio::test]
async fn a_permanent_delete_scrubs_the_local_copies_the_same_way_the_server_does()
-> Result<(), String> {
    let rig = rig("sync_purge_effect").await?;
    create_bot(&rig).await?;
    let dir_a = test_harness::test_dir("sync_purge_effect_a")?;
    let dir_b = test_harness::test_dir("sync_purge_effect_b")?;
    let a = Arc::new(rig.client(&dir_a, Counting::arc(0)).await?);
    let b = rig.client(&dir_b, Counting::arc(1)).await?;
    let record = NewRecord {
        stream: bot(),
        kind: "message".to_owned(),
        content: message("secret words"),
        task_id: None,
    };
    a.add_record(record).await.map_err(|e| e.to_string())?;
    until(&rig, "both hold the message", || async {
        a.pull_now(&bot());
        b.pull_now(&bot());
        let (mine, theirs) = (a.records(&bot()).await, b.records(&bot()).await);
        Ok(mine.is_ok_and(|rows| rows.len() == 1) && theirs.is_ok_and(|rows| rows.len() == 1))
    })
    .await?;
    let stream = json!({"bot": {"id": BOT}});
    let delete = json!({"stream": stream, "action": "delete", "target_seq": 1});
    let reply = ask(&a, &rig, "delete-1", delete)
        .await?
        .map_err(|e| e.to_string())?;
    assert_eq!(reply.status, 200);
    let deleted_at = serde_json::from_slice::<Value>(&reply.body)
        .map_err(|e| e.to_string())?
        .get("seq")
        .and_then(Value::as_i64)
        .ok_or("no seq in the reply")?;
    let purge = json!({"stream": stream, "action": "purge_permanent", "target_seq": deleted_at});
    let reply = ask(&a, &rig, "purge-1", purge)
        .await?
        .map_err(|e| e.to_string())?;
    assert_eq!(reply.status, 200);
    until(&rig, "both replicas follow the server", || async {
        a.pull_now(&bot());
        b.pull_now(&bot());
        let server = rig.server_rows(BOT_STREAM_ID).await?;
        let (mine, theirs) = (a.records(&bot()).await, b.records(&bot()).await);
        let (Ok(mine), Ok(theirs)) = (mine, theirs) else {
            return Ok(false);
        };
        Ok(same_as_server(&mine, &server) && same_as_server(&theirs, &server))
    })
    .await?;
    let local = b.records(&bot()).await.map_err(|e| e.to_string())?;
    let first = local.first().ok_or("replica is empty")?;
    assert!(first.purged);
    assert!(!first.content.to_string().contains("secret"));
    drop(a);
    rig.served.stop().await
}
