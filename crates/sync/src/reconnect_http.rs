use std::sync::Arc;

use anybot_foundation::{Clock, TrustStore, TrustedCerts};
use url::Url;

use crate::reconnect_connect::{ConnectPhaseLayer, ConnectProbe};
use crate::reconnect_direct::goes_direct;
use crate::reconnect_pacing::CONNECT_PHASE;

pub struct SyncHttp {
    pub client: reqwest::Client,
    pub probe: Arc<ConnectProbe>,
    pub os_notes: Vec<String>,
}

pub fn build_sync_http(
    trusted: &TrustedCerts,
    proxy: Option<&str>,
    clock: Arc<dyn Clock>,
) -> Result<SyncHttp, String> {
    let store = TrustStore::new(trusted)?;
    let os_notes = store.os_notes().to_vec();
    let tls = store.client_config()?;
    let probe = Arc::new(ConnectProbe::default());
    let layer = ConnectPhaseLayer::new(clock, CONNECT_PHASE, Arc::clone(&probe));
    let mut builder = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .use_preconfigured_tls(tls)
        .no_proxy()
        .connector_layer(layer);
    if let Some(raw) = proxy {
        builder = builder.proxy(explicit_proxy(raw)?);
    }
    let client = builder.build().map_err(|err| err.to_string())?;
    Ok(SyncHttp {
        client,
        probe,
        os_notes,
    })
}

fn explicit_proxy(raw: &str) -> Result<reqwest::Proxy, String> {
    let parsed = Url::parse(raw).map_err(|err| format!("proxy url: {err}"))?;
    Ok(reqwest::Proxy::custom(move |target| {
        if goes_direct(target) {
            None
        } else {
            Some(parsed.clone())
        }
    }))
}
