use std::time::Duration;

use crate::GatewayFail;
use crate::catalog::{self, LIST_WAIT_SECS};
use crate::errors::{clip_detail, redact};
use crate::providers::key_allowed;
use crate::types::ProviderEndpoint;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ListOutcome {
    Names(Vec<String>),
    Empty,
    Failed { code: &'static str, detail: String },
}

pub async fn fetch_models(
    gateway: &crate::Gateway,
    endpoint: &ProviderEndpoint,
) -> Result<ListOutcome, GatewayFail> {
    if endpoint.api_key.is_none() && needs_key(endpoint.kind) {
        return Ok(ListOutcome::Failed {
            code: "model-missing-key",
            detail: String::new(),
        });
    }
    let url = format!("{}/models", endpoint.base_url);
    if endpoint.api_key.is_some() && !key_allowed(endpoint, &url) {
        return Err(GatewayFail::KeyDestination);
    }
    let builder = gateway.client_request(&url, &serde_json::json!({}), endpoint);
    let wait = Duration::from_secs(LIST_WAIT_SECS);
    match gateway.transport_send(builder, wait).await {
        Ok(bytes) if bytes.status >= 400 => Ok(ListOutcome::Failed {
            code: "model-list-status",
            detail: redact(
                &clip_detail(&String::from_utf8_lossy(&bytes.body)),
                endpoint
                    .api_key
                    .as_ref()
                    .map(anybot_foundation::Secret::expose),
            ),
        }),
        Ok(bytes) => {
            let names = parse_ids(&String::from_utf8_lossy(&bytes.body));
            if names.is_empty() {
                Ok(ListOutcome::Empty)
            } else {
                Ok(ListOutcome::Names(catalog::normalize_ids(&names)))
            }
        }
        Err(
            crate::transport::TransportFail::FirstToken
            | crate::transport::TransportFail::ConnectTimeout,
        ) => Ok(ListOutcome::Failed {
            code: catalog::list_timeout_code(),
            detail: "timeout".to_owned(),
        }),
        Err(other) => Err(GatewayFail::Transport(other)),
    }
}

fn parse_ids(body: &str) -> Vec<String> {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(body) else {
        return Vec::new();
    };
    let Some(rows) = value.get("data").and_then(|data| data.as_array()) else {
        return Vec::new();
    };
    rows.iter()
        .filter_map(|row| row.get("id").and_then(|id| id.as_str()).map(str::to_owned))
        .collect()
}

fn needs_key(kind: crate::types::ProviderKind) -> bool {
    !matches!(
        kind,
        crate::types::ProviderKind::Ollama | crate::types::ProviderKind::Custom
    )
}
