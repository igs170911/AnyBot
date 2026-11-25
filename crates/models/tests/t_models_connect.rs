use proptest as _;
use serde as _;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use trybuild as _;

use anybot_foundation::clock::Clock;
use anybot_models::{ProviderKind, ThinkingMode};
use bytes as _;
use futures_util as _;
use reqwest as _;
use serde_json as _;
use test_harness::{
    FakeClock, NameMap, net_attach, net_hang_connect, net_start, sleep_real, timeout_real,
};
use tokio as _;
use url as _;

#[path = "../test_support/ask.rs"]
mod support;

#[tokio::test]
async fn gateway_connect_timeout_uses_clock() -> Result<(), String> {
    let (ctl, proxy_port) = net_start().await.unwrap();
    let real = SocketAddr::from(([127, 0, 0, 1], proxy_port));
    net_attach(&ctl, proxy_port, real).await.unwrap();
    net_hang_connect(&ctl, proxy_port).await.unwrap();
    let clock = FakeClock::new(0);
    let mut map = NameMap::new();
    map.insert(
        "api.cloud.test",
        SocketAddr::from(([127, 0, 0, 1], proxy_port)),
    );
    let gateway = anybot_models::gateway(as_clock(&clock))?.with_names(Arc::new(MapNames(map)));
    let endpoint = anybot_models::resolve_endpoint(
        ProviderKind::OpenAi,
        "http://api.cloud.test/v1",
        "k",
        "gpt-5",
        None,
    );
    let task = tokio::spawn(async move {
        gateway
            .complete(
                &endpoint,
                &support::hi(ThinkingMode::Auto),
                false,
                &anybot_models::SendPermit::for_endpoint(
                    &anybot_models::DataPolicy::Cloud {
                        allowed: vec![ProviderKind::OpenAi],
                    },
                    &endpoint,
                    anybot_models::ModelUse::Bot,
                )
                .expect("permit"),
            )
            .await
    });
    let ready = timeout_real(wait_clock(&clock, &task, 16), Duration::from_secs(5)).await;
    assert!(ready.is_ok());
    let err = task.await.unwrap().unwrap_err();
    assert!(matches!(
        err,
        anybot_models::GatewayFail::Transport(anybot_models::TransportFail::ConnectTimeout)
    ));
    Ok(())
}

async fn wait_clock(
    clock: &FakeClock,
    task: &tokio::task::JoinHandle<Result<anybot_models::TurnResult, anybot_models::GatewayFail>>,
    secs: u64,
) {
    loop {
        sleep_real(Duration::from_millis(30)).await;
        let _woke = clock.advance(Duration::from_secs(secs));
        if task.is_finished() {
            break;
        }
    }
}

fn as_clock(clock: &Arc<FakeClock>) -> Arc<dyn Clock> {
    let shared: Arc<FakeClock> = Arc::clone(clock);
    shared
}

struct MapNames(NameMap);

impl anybot_models::NameResolver for MapNames {
    fn resolve(&self, url: &str) -> Option<String> {
        self.0.resolve(url)
    }
}
