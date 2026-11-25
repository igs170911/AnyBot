use std::sync::Arc;

use anybot_foundation::clock::Clock;
use tokio::sync::watch;

use crate::ask::{self, AttemptLimit};
use crate::capabilities::{self, Learned};
use crate::egress::{self, DataPolicy, ModelUse};
use crate::handoff::{HandoffRecord, SwitchReason};
use crate::notice::{Notice, skip_notice};
use crate::types::{Audience, ProviderEndpoint, ProviderKind, TurnRequest, TurnResult};
use crate::{Gateway, GatewayFail};

#[must_use]
pub fn ask_plan<'a>(
    primary: &'a ModelRef,
    fallbacks: &'a [ModelRef],
    policy: &'a DataPolicy,
    request: &'a TurnRequest,
    audience: Audience,
) -> AskPlan<'a> {
    AskPlan {
        primary,
        fallbacks,
        policy,
        request,
        request_for: None,
        stream: false,
        audience,
        bot: "bot",
    }
}

pub struct AskPlan<'a> {
    pub primary: &'a ModelRef,
    pub fallbacks: &'a [ModelRef],
    pub policy: &'a DataPolicy,
    pub request: &'a TurnRequest,
    pub request_for: Option<&'a mut (dyn FnMut(&ProviderEndpoint) -> TurnRequest + Send)>,
    pub stream: bool,
    pub audience: Audience,
    pub bot: &'a str,
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

pub struct Session {
    gateway: Gateway,
    clock: Arc<dyn Clock>,
    learned: Vec<Learned>,
    pub(crate) resting: Vec<RestMark>,
    jitter_milli: i32,
    stop: watch::Receiver<bool>,
    pub(crate) pending: Option<(ProviderKind, String)>,
    pub(crate) pending_reason: Option<SwitchReason>,
    pub(crate) active: Option<(ProviderKind, String)>,
    pub(crate) failed: Option<(ProviderKind, String)>,
    pub(crate) failed_reason: Option<SwitchReason>,
    pub(crate) handoffs: Vec<HandoffRecord>,
    pub(crate) notices: Vec<Notice>,
}

impl Session {
    pub fn new(gateway: Gateway, clock: Arc<dyn Clock>, jitter_milli: i32) -> Self {
        let (_tx, stop) = watch::channel(false);
        Self {
            gateway,
            clock,
            learned: Vec::new(),
            resting: Vec::new(),
            jitter_milli,
            stop,
            pending: None,
            pending_reason: None,
            active: None,
            failed: None,
            failed_reason: None,
            handoffs: Vec::new(),
            notices: Vec::new(),
        }
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

    pub fn request_switch(&mut self, provider: ProviderKind, model: &str, reason: SwitchReason) {
        self.pending = Some((provider, model.to_owned()));
        self.pending_reason = Some(reason);
    }

    #[must_use]
    pub fn switched_model(&self) -> Option<(ProviderKind, &str)> {
        self.active
            .as_ref()
            .map(|(kind, model)| (*kind, model.as_str()))
    }

    pub async fn ask(&mut self, plan: &mut AskPlan<'_>) -> Result<AskOutcome, GatewayFail> {
        let chain =
            crate::session_chain::chain_of(plan.primary, plan.fallbacks, self.pending.as_ref());
        let request = plan.request;
        let policy = plan.policy;
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
            if !egress::allowed(
                policy,
                &model.endpoint.base_url,
                model.endpoint.kind,
                ModelUse::Bot,
            ) {
                skips.push(skip_notice(model.endpoint.kind, bot));
                continue;
            }
            if model.endpoint.api_key.is_none() && needs_key(model.endpoint.kind) {
                skips.push(crate::notice::missing_key_notice(model.endpoint.kind));
                continue;
            }
            crate::session_switch::note_active(self, &model.endpoint);
            let built = crate::session_step::request_for_model(
                model,
                plan.primary,
                request,
                &mut plan.request_for,
            );
            match self.one(model, &built, stream, audience).await {
                Ok(result) => return Ok(crate::session_switch::finish_ask(self, result, skips)),
                Err(err) => match crate::session_switch::keep_or_stop(self, model, err) {
                    crate::session_switch::ChainStep::Next(kept) => last = Some(kept),
                    crate::session_switch::ChainStep::Stop(err) => return Err(err),
                },
            }
        }
        Err(match last {
            Some(err) => err,
            None => ask::classified_fail(
                crate::errors::classify(&crate::errors::FailInput::MissingKey),
                std::mem::take(&mut self.notices),
            ),
        })
    }

    async fn one(
        &mut self,
        model: &ModelRef,
        request: &TurnRequest,
        stream: bool,
        audience: Audience,
    ) -> Result<TurnResult, GatewayFail> {
        let cap = capabilities::resolve(
            model.endpoint.kind,
            &model.endpoint.model,
            model.bot_context,
            self.learned_for(&model.endpoint),
        );
        if !cap.tools && !request.tools.is_empty() {
            return Err(crate::session_step::no_tools_fail());
        }
        let usable = capabilities::usable_context(
            cap.context_tokens,
            model.bot_context,
            self.learned_for(&model.endpoint)
                .and_then(|row| row.context_tokens),
            request.max_output_tokens.unwrap_or(0),
        );
        let calibration = match self.learned_for(&model.endpoint) {
            Some(row) => row.calibration.clone(),
            None => crate::estimate::Calibration::new(),
        };
        if crate::session_step::over_budget(request, usable, &calibration) {
            return Err(crate::session_step::budget_fail());
        }
        crate::session_rest::send_tries(
            self,
            model,
            request,
            stream,
            audience,
            &calibration,
            cap.thinking,
        )
        .await
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

    fn learned_for(&self, endpoint: &ProviderEndpoint) -> Option<&Learned> {
        crate::session_rest::learned_for(&self.learned, endpoint)
    }
}

fn needs_key(kind: ProviderKind) -> bool {
    !matches!(kind, ProviderKind::Ollama | ProviderKind::Custom)
}
