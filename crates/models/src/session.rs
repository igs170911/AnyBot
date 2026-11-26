use std::sync::{Arc, Mutex};

use anybot_foundation::clock::Clock;
use tokio::sync::watch;

use crate::ask::{self, AttemptLimit};
use crate::capabilities::Learned;
use crate::egress::{self, DataPolicy, ModelUse};
use crate::handoff::{HandoffRecord, SwitchReason};
use crate::notice::{Notice, skip_notice};
use crate::types::{Audience, ProviderEndpoint, ProviderKind, TurnRequest, TurnResult};
use crate::{Gateway, GatewayFail};

#[must_use]
pub fn ask_plan<'a>(
    primary: &'a ModelRef,
    fallbacks: &'a [ModelRef],
    request: &'a TurnRequest,
    audience: Audience,
) -> AskPlan<'a> {
    AskPlan {
        primary,
        fallbacks,
        request,
        request_for: None,
        stream: false,
        audience,
        bot: "bot",
        use_kind: ModelUse::Bot,
    }
}

pub struct AskPlan<'a> {
    pub primary: &'a ModelRef,
    pub fallbacks: &'a [ModelRef],
    pub request: &'a TurnRequest,
    pub request_for: Option<&'a mut (dyn FnMut(&ProviderEndpoint) -> TurnRequest + Send)>,
    pub stream: bool,
    pub audience: Audience,
    pub bot: &'a str,
    pub use_kind: ModelUse,
}

pub const REST_MS: u64 = 10 * 60 * 1_000;
pub const MAX_FALLBACKS: usize = 8;

pub struct ModelRef {
    pub endpoint: ProviderEndpoint,
    pub bot_context: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RestMark {
    pub key: String,
    pub until_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AskOutcome {
    pub result: TurnResult,
    pub notices: Vec<Notice>,
    pub handoffs: Vec<HandoffRecord>,
    pub skips: Vec<Notice>,
}

pub struct SwitchHandle {
    pending: Arc<Mutex<Option<(ModelRef, SwitchReason)>>>,
}

impl SwitchHandle {
    pub fn request(&self, model: &ModelRef, reason: SwitchReason) {
        match self.pending.lock() {
            Ok(mut slot) => *slot = Some((crate::model_copy::copy_model(model), reason)),
            Err(poisoned) => {
                *poisoned.into_inner() = Some((crate::model_copy::copy_model(model), reason));
            }
        }
    }
}

pub struct Session {
    gateway: Gateway,
    policy: DataPolicy,
    pub(crate) clock: Arc<dyn Clock>,
    learned: Vec<Learned>,
    pub(crate) resting: Vec<RestMark>,
    pub(crate) jitter_milli: i32,
    pub(crate) stop: watch::Receiver<bool>,
    pub(crate) pending: Arc<Mutex<Option<(ModelRef, SwitchReason)>>>,
    pub(crate) active: Option<ModelRef>,
    pub(crate) failed: Option<ModelRef>,
    pub(crate) failed_reason: Option<SwitchReason>,
    pub(crate) failed_code: Option<String>,
    pub(crate) adopted: Option<ModelRef>,
    pub(crate) primary_override: Option<ModelRef>,
    pub(crate) handoffs: Vec<HandoffRecord>,
    pub(crate) notices: Vec<Notice>,
    pub(crate) logs: Vec<crate::failure_log::FailureLog>,
    last_estimate: Option<u64>,
}

impl Session {
    pub fn new(
        gateway: Gateway,
        policy: DataPolicy,
        clock: Arc<dyn Clock>,
        jitter_milli: i32,
    ) -> Self {
        let (_tx, stop) = watch::channel(false);
        Self {
            gateway,
            policy,
            clock,
            learned: Vec::new(),
            resting: Vec::new(),
            jitter_milli,
            stop,
            pending: Arc::new(Mutex::new(None)),
            active: None,
            failed: None,
            failed_reason: None,
            failed_code: None,
            adopted: None,
            primary_override: None,
            handoffs: Vec::new(),
            notices: Vec::new(),
            logs: Vec::new(),
            last_estimate: None,
        }
    }

    #[must_use]
    pub fn failure_logs(&self) -> &[crate::failure_log::FailureLog] {
        &self.logs
    }

    #[must_use]
    pub fn with_stop(mut self, stop: watch::Receiver<bool>) -> Self {
        self.stop = stop;
        self
    }

    pub fn load_learned(&mut self, learned: Vec<Learned>) {
        self.learned = learned;
    }

    pub fn load_rest(&mut self, resting: Vec<RestMark>) {
        self.resting = resting;
    }

    #[must_use]
    pub fn learned(&self) -> &[Learned] {
        &self.learned
    }

    #[must_use]
    pub fn resting(&self) -> &[RestMark] {
        &self.resting
    }

    pub fn request_switch(&self, model: &ModelRef, reason: SwitchReason) {
        self.switch_handle().request(model, reason);
    }

    #[must_use]
    pub fn switch_handle(&self) -> SwitchHandle {
        SwitchHandle {
            pending: Arc::clone(&self.pending),
        }
    }

    #[must_use]
    pub fn policy(&self) -> &DataPolicy {
        &self.policy
    }

    #[must_use]
    pub fn switched_model(&self) -> Option<(ProviderKind, &str)> {
        self.active
            .as_ref()
            .map(|model| (model.endpoint.kind, model.endpoint.model.as_str()))
    }

    pub async fn ask(&mut self, plan: &mut AskPlan<'_>) -> Result<AskOutcome, GatewayFail> {
        if let Some(next) = self.adopted.as_ref() {
            self.primary_override = Some(crate::model_copy::copy_model(next));
        }
        let pending = crate::session_io::read_pending(self).map(|(model, _)| model);
        let chain = crate::session_chain::chain_of(
            plan.primary,
            plan.fallbacks,
            pending.as_ref(),
            self.primary_override.as_ref(),
        );
        let request = plan.request;
        let stream = plan.stream;
        let audience = plan.audience;
        let bot = plan.bot;
        let mut skips = Vec::new();
        let mut last = None;
        for model in chain {
            if crate::session_rest::resting_now(
                &self.resting,
                &model.endpoint,
                self.clock.now_ms().millis(),
            ) {
                continue;
            }
            let Some(permit) = egress::SendPermit::for_endpoint(
                &self.policy.clone(),
                &model.endpoint,
                plan.use_kind,
            ) else {
                skips.push(skip_notice(model.endpoint.kind, bot));
                continue;
            };
            if model.endpoint.api_key.is_none() && needs_key(model.endpoint.kind) {
                skips.push(crate::notice::missing_key_notice(model.endpoint.kind));
                continue;
            }
            crate::session_switch::note_active(self, &model);
            let built = crate::session_step::request_for_model(
                &model,
                plan.primary,
                request,
                &mut plan.request_for,
            );
            match crate::session_ask::one(self, &model, &built, stream, audience, &permit).await {
                Ok(result) => {
                    return Ok(crate::session_switch::finish_ask(self, result, skips));
                }
                Err(err) => match crate::session_switch::keep_or_stop(self, &model, err) {
                    crate::session_switch::ChainStep::Next(kept) => last = Some(kept),
                    crate::session_switch::ChainStep::Stop(err) => return Err(err),
                },
            }
        }
        Err(crate::session_io::exhausted(last, skips, &self.resting))
    }

    pub(crate) async fn wait_or_stop(
        &self,
        fail: &mut ask::AttemptFail,
        sent: u32,
        audience: Audience,
    ) -> Result<bool, GatewayFail> {
        ask::wait_retry(
            &self.clock,
            &self.stop,
            fail,
            sent,
            AttemptLimit {
                audience,
                jitter_milli: self.jitter_milli,
            },
        )
        .await
    }

    pub(crate) fn gateway(&self) -> &Gateway {
        &self.gateway
    }

    pub(crate) fn learned_rows(&mut self) -> &mut Vec<Learned> {
        &mut self.learned
    }

    pub(crate) fn notices_mut(&mut self) -> &mut Vec<Notice> {
        &mut self.notices
    }

    pub(crate) fn clock_now(&self) -> u64 {
        self.clock.now_ms().millis()
    }

    pub(crate) fn remember_estimate(&mut self, tokens: u64) {
        self.last_estimate = Some(tokens);
    }

    #[must_use]
    pub fn last_estimate(&self) -> Option<u64> {
        self.last_estimate
    }
}

fn needs_key(kind: ProviderKind) -> bool {
    !matches!(kind, ProviderKind::Ollama | ProviderKind::Custom)
}
