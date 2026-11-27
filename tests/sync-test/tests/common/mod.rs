use anybot_domain as _;
use anybot_server_test as _;

use std::future::Future;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use anybot_app_test::clockwalk::ClockWalker;
use anybot_app_test::http::{RequestSpec, post};
use anybot_app_test::serve::{Served, boot_pair, serve_start};
use anybot_foundation::{Clock, Secret, TrustedCerts};
use anybot_sync::{RandomSource, RandomUnavailable, SyncClient, SyncConfig};
use serde_json::{Value, json};
use sqlx::Row as _;
use test_harness::netctl::{net_attach, net_start};
use test_harness::{FakeClock, NetCtl, sleep_real};

pub(crate) const CLIENT_VERSION: &str = "0.1.0";
pub(crate) const ACCOUNT_STREAM_ID: &str = "account/alice";

pub(crate) struct Rig {
    pub(crate) served: Served,
    pub(crate) ctl: Arc<NetCtl>,
    pub(crate) proxy_port: u16,
    pub(crate) bearer: String,
}

pub(crate) struct Counting(AtomicU64);

impl Counting {
    pub(crate) fn arc(start: u64) -> Arc<dyn RandomSource> {
        Arc::new(Counting(AtomicU64::new(start)))
    }
}

impl RandomSource for Counting {
    fn next_u64(&self) -> Result<u64, RandomUnavailable> {
        Ok(self.0.fetch_add(1, Ordering::SeqCst))
    }
}

pub(crate) async fn rig(test: &str) -> Result<Rig, String> {
    let served = serve_start(test).await?;
    boot_pair(&served).await?;
    let bearer = login(&served).await?;
    let (ctl, proxy_port) = net_start().await?;
    net_attach(&ctl, proxy_port, served.origin_addr()).await?;
    Ok(Rig {
        served,
        ctl,
        proxy_port,
        bearer,
    })
}

async fn login(served: &Served) -> Result<String, String> {
    let body = json!({
        "account": "alice",
        "password": "alicepassword",
        "label": "test-device",
        "credential": "bearer",
    })
    .to_string();
    let spec = RequestSpec::new(&body).with_key("k-sync-login");
    let mut walker = ClockWalker::start(&served.clock);
    let answer = post(&served.base_url(), "/v1/account/login", &spec).await;
    walker.stop().await?;
    answer?.bearer()
}

impl Rig {
    pub(crate) fn clock(&self) -> Arc<FakeClock> {
        Arc::clone(&self.served.clock)
    }

    pub(crate) fn clock_dyn(&self) -> Arc<dyn Clock> {
        let clock: Arc<dyn Clock> = self.clock();
        clock
    }

    pub(crate) fn url(&self) -> String {
        format!("http://127.0.0.1:{}", self.proxy_port)
    }

    pub(crate) async fn client(
        &self,
        dir: &Path,
        random: Arc<dyn RandomSource>,
    ) -> Result<SyncClient, String> {
        self.client_at(&self.url(), dir, random).await
    }

    pub(crate) async fn client_at(
        &self,
        url: &str,
        dir: &Path,
        random: Arc<dyn RandomSource>,
    ) -> Result<SyncClient, String> {
        self.client_as(&self.bearer, url, dir, random).await
    }

    pub(crate) async fn client_as(
        &self,
        bearer: &str,
        url: &str,
        dir: &Path,
        random: Arc<dyn RandomSource>,
    ) -> Result<SyncClient, String> {
        let config = SyncConfig {
            data_dir: dir.to_path_buf(),
            server_url: url.to_owned(),
            credential: Secret::from_untrusted(bearer.to_owned()),
            client_version: CLIENT_VERSION.to_owned(),
            trusted: TrustedCerts::empty(),
            proxy: None,
            clock: self.clock_dyn(),
            random,
        };
        SyncClient::start(config).await.map_err(|e| e.to_string())
    }

    pub(crate) async fn server_rows(&self, stream_id: &str) -> Result<Vec<ServerRow>, String> {
        let rows = sqlx::query(
            "SELECT seq, device_id, kind, content, offline_sent FROM records
                WHERE stream_id = $1 ORDER BY seq",
        )
        .bind(stream_id)
        .fetch_all(&self.served.pool)
        .await
        .map_err(|e| e.to_string())?;
        rows.iter().map(row_of).collect()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ServerRow {
    pub(crate) seq: i64,
    pub(crate) device_id: String,
    pub(crate) kind: String,
    pub(crate) content: Value,
    pub(crate) offline_sent: bool,
}

fn row_of(row: &sqlx::postgres::PgRow) -> Result<ServerRow, String> {
    let text: String = row.try_get("content").map_err(|e| e.to_string())?;
    Ok(ServerRow {
        seq: row.try_get("seq").map_err(|e| e.to_string())?,
        device_id: row.try_get("device_id").map_err(|e| e.to_string())?,
        kind: row.try_get("kind").map_err(|e| e.to_string())?,
        content: serde_json::from_str(&text).map_err(|e| e.to_string())?,
        offline_sent: row.try_get("offline_sent").map_err(|e| e.to_string())?,
    })
}

pub(crate) async fn wait_until<F, Fut>(what: &str, mut check: F) -> Result<(), String>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<bool, String>>,
{
    for _round in 0..1000 {
        if check().await? {
            return Ok(());
        }
        sleep_real(Duration::from_millis(20)).await;
    }
    Err(format!("gave up waiting for {what}"))
}

pub(crate) fn message(text: &str) -> Value {
    json!({"text": text})
}

pub(crate) async fn drive_until<F, Fut>(
    what: &str,
    clock: &Arc<FakeClock>,
    step: Duration,
    mut check: F,
) -> Result<(), String>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<bool, String>>,
{
    for _round in 0..3000 {
        if check().await? {
            return Ok(());
        }
        sleep_real(Duration::from_millis(10)).await;
        let _woken = clock.advance(step);
    }
    Err(format!("gave up driving the clock until {what}"))
}
