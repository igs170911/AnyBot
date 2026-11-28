use std::sync::{Arc, Mutex};

use crate::fake_clock::FakeClock;
use crate::model_named::NamedScripts;
use crate::model_request::requested_model;
use crate::model_script::{
    RecordedRequest, Script, Step, classify_key, presented_key, record_headers,
};

#[derive(Default)]
struct ModelInner {
    script: Script,
    named: NamedScripts,
    recorded: Vec<RecordedRequest>,
    format: ModelFormat,
    expected_key: Option<String>,
    header_wait: Option<Arc<FakeClock>>,
}

#[derive(Default)]
pub struct FakeModel {
    inner: Mutex<ModelInner>,
    arrived: tokio::sync::watch::Sender<usize>,
}

impl FakeModel {
    pub fn script_set(&self, steps: Vec<Step>) {
        let Ok(mut inner) = self.inner.lock() else {
            return;
        };
        inner.script.clear();
        for step in steps {
            inner.script.push(step);
        }
    }

    pub fn script_set_for(&self, model: &str, steps: Vec<Step>) {
        let Ok(mut inner) = self.inner.lock() else {
            return;
        };
        inner.named.set(model, steps);
    }

    pub fn script_len_for(&self, model: &str) -> usize {
        let Ok(inner) = self.inner.lock() else {
            return 0;
        };
        inner.named.len(model)
    }

    pub fn recorded_for(&self, model: &str) -> Vec<RecordedRequest> {
        self.recorded()
            .into_iter()
            .filter(|request| requested_model(&request.body) == model)
            .collect()
    }

    pub fn script_clear(&self) {
        let Ok(mut inner) = self.inner.lock() else {
            return;
        };
        inner.script.clear();
    }

    pub fn script_len(&self) -> usize {
        let Ok(inner) = self.inner.lock() else {
            return 0;
        };
        inner.script.len()
    }

    pub fn format_set(&self, format: ModelFormat) {
        let Ok(mut inner) = self.inner.lock() else {
            return;
        };
        inner.format = format;
    }

    pub fn header_clock(&self, clock: Arc<FakeClock>) {
        let Ok(mut inner) = self.inner.lock() else {
            return;
        };
        inner.header_wait = Some(clock);
    }

    pub fn expect_key(&self, key: &str) {
        let Ok(mut inner) = self.inner.lock() else {
            return;
        };
        inner.expected_key = Some(key.to_owned());
    }

    #[must_use]
    pub fn format(&self) -> ModelFormat {
        let Ok(inner) = self.inner.lock() else {
            return ModelFormat::OpenAi;
        };
        inner.format
    }

    pub fn recorded(&self) -> Vec<RecordedRequest> {
        let Ok(inner) = self.inner.lock() else {
            return Vec::new();
        };
        inner.recorded.clone()
    }

    pub async fn wait_recorded(&self, count: usize) -> Result<(), String> {
        let mut seen = self.arrived.subscribe();
        seen.wait_for(|total| *total >= count)
            .await
            .map(|_total| ())
            .map_err(|_e| "fake model stopped recording requests".to_owned())
    }

    fn record(&self, path: &str, body: String, headers: &axum::http::HeaderMap) {
        let pairs: Vec<(String, String)> = headers
            .iter()
            .map(|(name, value)| {
                (
                    name.as_str().to_owned(),
                    value.to_str().unwrap_or_default().to_owned(),
                )
            })
            .collect();
        let presented = pairs
            .iter()
            .find_map(|(name, value)| presented_key(name, value));
        let recorded = record_headers(pairs.into_iter());
        let Ok(mut inner) = self.inner.lock() else {
            return;
        };
        let key_match = classify_key(inner.expected_key.as_deref(), presented.as_deref());
        inner.recorded.push(RecordedRequest {
            path: path.to_owned(),
            body,
            headers: recorded,
            key_match,
        });
        self.arrived.send_replace(inner.recorded.len());
    }

    fn header_wait(&self) -> Option<Arc<FakeClock>> {
        let Ok(inner) = self.inner.lock() else {
            return None;
        };
        inner.header_wait.clone()
    }

    fn take_step(&self, model: &str) -> Option<Step> {
        let Ok(mut inner) = self.inner.lock() else {
            return None;
        };
        match inner.named.take(model) {
            Some(own) => own,
            None => inner.script.take(),
        }
    }
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum ModelFormat {
    #[default]
    OpenAi,
    Anthropic,
    Vllm,
}

mod routes;

pub use routes::{model_router, model_start, model_start_with_format};
