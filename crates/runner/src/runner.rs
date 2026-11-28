use std::sync::Arc;

use anybot_foundation::TurnId;
use anybot_foundation::{BotId, Clock, DeviceId, TaskId};
use anybot_messages::Lang;
use anybot_models::{Calibration, Capability, ModelRef, Session, ThinkingMode, ToolSpec};
use anybot_protocol::{
    BudgetItem, ComputerRef, KnownTaskEventKind, TaskOpener, TurnEndReason, TurnInitiator,
};

use crate::card_wait::TaskWatch;
use crate::error::TurnError;
use crate::fallback::ModelReach;
use crate::record::{RawStore, Record};
use crate::steer::{ActiveSteer, ClearOnDrop, Steer, SteerQueued};
use crate::step::{StepFlow, run_step};

pub type BudgetLimitValue = u64;

pub struct BotProfile {
    pub id: BotId,
    pub name: String,
    pub instructions: String,
    pub timezone: String,
    pub budget_overrides: Vec<(BudgetItem, BudgetLimitValue)>,
}

pub struct TurnModel {
    pub primary: ModelRef,
    pub fallbacks: Vec<ModelRef>,
    pub thinking: ThinkingMode,
    pub capability: Capability,
    pub calibration: Calibration,
    pub reserved_output: u64,
}

pub struct RunnerParts {
    pub gated: Arc<dyn anybot_trust::GatedComputer>,
    pub record: Arc<dyn Record>,
    pub raw: Arc<dyn RawStore>,
    pub watch: Arc<dyn TaskWatch>,
    pub session: Session,
    pub clock: Arc<dyn Clock>,
    pub bot: BotProfile,
    pub model: TurnModel,
    pub lang: Option<Lang>,
    pub computer: ComputerRef,
    pub model_reach: Arc<dyn ModelReach>,
    pub trust: Arc<dyn crate::trust_settings::TrustSettings>,
}

pub struct TurnCtx {
    pub(crate) task: TaskId,
    pub(crate) turn: u32,
    pub(crate) turn_id: TurnId,
    pub(crate) turn_started: u64,
    pub(crate) initiator: TurnInitiator,
    pub(crate) evidence: tokio::sync::Mutex<u32>,
    pub(crate) active_model: std::sync::Mutex<Option<TurnModel>>,
    pub(crate) tools: Vec<ToolSpec>,
}

pub struct UserMessage {
    pub device: DeviceId,
    pub text: String,
}

pub struct TurnOutcome {
    pub task: TaskId,
    pub turn: u32,
    pub reason: TurnEndReason,
    pub pending: Vec<crate::approvals::PendingCard>,
    pub unread_steers: Vec<String>,
}

pub struct TurnRunner {
    pub(crate) gated: Arc<dyn anybot_trust::GatedComputer>,
    pub(crate) record: Arc<dyn Record>,
    pub(crate) raw: Arc<dyn RawStore>,
    pub(crate) watch: Arc<dyn TaskWatch>,
    pub(crate) session: Arc<tokio::sync::Mutex<Session>>,
    pub(crate) clock: Arc<dyn Clock>,
    pub(crate) bot: BotProfile,
    pub(crate) model: TurnModel,
    pub(crate) lang: Lang,
    pub(crate) computer: ComputerRef,
    pub(crate) model_reach: Arc<dyn ModelReach>,
    pub(crate) trust: Arc<dyn crate::trust_settings::TrustSettings>,
    pub(crate) conversation: std::sync::atomic::AtomicU64,
    active: Arc<tokio::sync::Mutex<()>>,
    pub(crate) parked: std::sync::Mutex<Vec<crate::approvals::PendingCard>>,
    pub(crate) pending_question: std::sync::Mutex<Option<crate::questions::PendingQuestion>>,
    pub(crate) last_ctx: std::sync::Mutex<Option<TurnCtx>>,
    pub(crate) steer_slot: std::sync::Mutex<Option<ActiveSteer>>,
    pub(crate) rejects: crate::reject_memory::RejectMemory,
}

impl TurnRunner {
    #[must_use]
    pub fn new(parts: RunnerParts) -> Self {
        let lang = crate::fixed_blocks::lang_of(parts.lang);
        TurnRunner {
            gated: parts.gated,
            record: parts.record,
            raw: parts.raw,
            watch: parts.watch,
            session: Arc::new(tokio::sync::Mutex::new(parts.session)),
            clock: parts.clock,
            bot: parts.bot,
            model: parts.model,
            lang,
            computer: parts.computer,
            model_reach: parts.model_reach,
            trust: parts.trust,
            conversation: std::sync::atomic::AtomicU64::new(0),
            active: Arc::new(tokio::sync::Mutex::new(())),
            parked: std::sync::Mutex::new(Vec::new()),
            pending_question: std::sync::Mutex::new(None),
            last_ctx: std::sync::Mutex::new(None),
            steer_slot: std::sync::Mutex::new(None),
            rejects: crate::reject_memory::RejectMemory::default(),
        }
    }

    #[must_use]
    pub fn conversation(&self) -> crate::trust_settings::ConversationKey {
        crate::trust_settings::ConversationKey(
            self.conversation.load(std::sync::atomic::Ordering::SeqCst),
        )
    }

    pub fn start_new_conversation(&self) {
        self.conversation
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }

    pub async fn on_user_message(&self, message: UserMessage) -> Result<TurnOutcome, TurnError> {
        let guard = self.lock_active()?;
        let _clear = ClearOnDrop(self);
        self.rejects.clear()?;
        let outcome = self.run_turn(message).await;
        drop(guard);
        outcome
    }

    pub fn steer(&self, steer: Steer) -> Result<SteerQueued, TurnError> {
        crate::steer::enqueue(self, steer)
    }

    pub(crate) fn lock_active(&self) -> Result<tokio::sync::OwnedMutexGuard<()>, TurnError> {
        Arc::clone(&self.active)
            .try_lock_owned()
            .map_err(|_busy| TurnError::Busy)
    }

    async fn run_turn(&self, message: UserMessage) -> Result<TurnOutcome, TurnError> {
        if let Some(arg) = crate::yolo_command::parse_command(&message.text) {
            return crate::yolo_command::run(self, &message, arg).await;
        }
        if let Some(pending) = crate::questions::take_parked(self) {
            return self.continue_with_unanswered(pending, message).await;
        }
        let task = TaskId::generate(&*self.clock);
        self.append_user_said(&task, &message).await?;
        let goal = message.text.clone();
        self.append_task_event(
            &task,
            KnownTaskEventKind::TaskCreated {
                goal,
                opened_by: TaskOpener::You,
                bot: self.bot.id,
                parent: None,
            },
        )
        .await?;
        self.append_budget_limits(&task).await?;
        let turn = self
            .count_task_kind(&task, KindCount::TurnStarted)
            .await?
            .saturating_add(1);
        let initiator = TurnInitiator::You {
            device: message.device,
        };
        let started = self
            .append_task_event_in_turn(&task, Some(turn), self.turn_started_kind(turn, initiator))
            .await?;
        let caps = self.gated.declared_caps();
        let tools = crate::tool_specs::turn_tools(self.lang, &caps)?;
        let ctx = TurnCtx {
            task,
            turn,
            turn_id: TurnId::generate(&*self.clock),
            turn_started: started.stream_seq,
            initiator,
            evidence: tokio::sync::Mutex::new(1),
            active_model: std::sync::Mutex::new(None),
            tools,
        };
        crate::steer::begin_turn(self, task, turn, true);
        let reason = self.run_turn_loop(&ctx).await?;
        crate::steer::finish_or_continue(self, ctx, reason).await
    }

    pub(crate) fn save_last_ctx(&self, ctx: &TurnCtx) {
        if let Ok(mut guard) = self.last_ctx.lock() {
            *guard = Some(ctx.clone());
        }
    }

    pub(crate) async fn run_turn_loop(&self, ctx: &TurnCtx) -> Result<TurnEndReason, TurnError> {
        loop {
            match run_step(self, ctx).await? {
                StepFlow::Continue => {}
                StepFlow::Ended(reason) => return Ok(reason),
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KindCount {
    TurnStarted,
    ModelAsked,
    TurnEnded,
}
