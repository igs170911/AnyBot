use std::sync::Arc;

use anybot_foundation::TurnId;
use anybot_foundation::{BotId, Clock, DeviceId, EventId, RecordId, TaskId};
use anybot_messages::Lang;
use anybot_models::{Calibration, Capability, ModelRef, Session, ThinkingMode, ToolSpec};
use anybot_protocol::{
    BudgetItem, ComputerRef, KnownTaskEventKind, TaskEventKind, TaskOpener, TurnEndReason,
    TurnInitiator,
};

use crate::card_wait::TaskWatch;
use crate::error::TurnError;
use crate::fallback::ModelReach;
use crate::record::{Appended, NewTaskEvent, RawStore, Record, RecordEntry, Stored, UserLine};
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
    pub tools: Vec<ToolSpec>,
    pub lang: Option<Lang>,
    pub computer: ComputerRef,
    pub model_reach: Arc<dyn ModelReach>,
}

pub struct TurnCtx {
    pub(crate) task: TaskId,
    pub(crate) turn: u32,
    pub(crate) turn_id: TurnId,
    pub(crate) turn_started: u64,
    pub(crate) initiator: TurnInitiator,
    pub(crate) evidence: tokio::sync::Mutex<u32>,
    pub(crate) active_model: std::sync::Mutex<Option<TurnModel>>,
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
    pub(crate) tools: Vec<ToolSpec>,
    pub(crate) lang: Lang,
    pub(crate) computer: ComputerRef,
    pub(crate) model_reach: Arc<dyn ModelReach>,
    active: Arc<tokio::sync::Mutex<()>>,
    pub(crate) parked: std::sync::Mutex<Vec<crate::approvals::PendingCard>>,
    pub(crate) last_ctx: std::sync::Mutex<Option<TurnCtx>>,
    pub(crate) steer_slot: std::sync::Mutex<Option<ActiveSteer>>,
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
            tools: parts.tools,
            lang,
            computer: parts.computer,
            model_reach: parts.model_reach,
            active: Arc::new(tokio::sync::Mutex::new(())),
            parked: std::sync::Mutex::new(Vec::new()),
            last_ctx: std::sync::Mutex::new(None),
            steer_slot: std::sync::Mutex::new(None),
        }
    }

    pub async fn on_user_message(&self, message: UserMessage) -> Result<TurnOutcome, TurnError> {
        let guard = self.lock_active()?;
        let _clear = ClearOnDrop(self);
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
        let ctx = TurnCtx {
            task,
            turn,
            turn_id: TurnId::generate(&*self.clock),
            turn_started: started.stream_seq,
            initiator,
            evidence: tokio::sync::Mutex::new(1),
            active_model: std::sync::Mutex::new(None),
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

    pub(crate) async fn append_budget_limits(&self, task: &TaskId) -> Result<(), TurnError> {
        for &(item, limit) in &self.bot.budget_overrides {
            self.append_task_event(task, KnownTaskEventKind::BudgetLimit { item, limit })
                .await?;
        }
        Ok(())
    }

    pub async fn append_task_event(
        &self,
        task: &TaskId,
        kind: KnownTaskEventKind,
    ) -> Result<Stored, TurnError> {
        self.append_task_event_in_turn(task, None, kind).await
    }

    pub(crate) async fn append_turn_event(
        &self,
        ctx: &TurnCtx,
        kind: KnownTaskEventKind,
    ) -> Result<Stored, TurnError> {
        self.append_task_event_in_turn(&ctx.task, Some(ctx.turn), kind)
            .await
    }

    pub(crate) async fn append_turn_unless_halted(
        &self,
        ctx: &TurnCtx,
        kind: KnownTaskEventKind,
    ) -> Result<Appended, TurnError> {
        let new = NewTaskEvent {
            bot: self.bot.id,
            task: ctx.task,
            turn: Some(ctx.turn),
            event_id: EventId::generate(&*self.clock),
            at_ms: self.clock.now_ms(),
            kind: TaskEventKind::Known(kind),
        };
        self.record
            .append_unless_halted(RecordEntry::Task(new), ctx.turn_started)
            .await
            .map_err(TurnError::Record)
    }

    pub(crate) async fn append_task_event_in_turn(
        &self,
        task: &TaskId,
        turn: Option<u32>,
        kind: KnownTaskEventKind,
    ) -> Result<Stored, TurnError> {
        let new = NewTaskEvent {
            bot: self.bot.id,
            task: *task,
            turn,
            event_id: EventId::generate(&*self.clock),
            at_ms: self.clock.now_ms(),
            kind: TaskEventKind::Known(kind),
        };
        self.record
            .append(RecordEntry::Task(new))
            .await
            .map_err(TurnError::Record)
    }

    pub(crate) async fn append_user_said(
        &self,
        task: &TaskId,
        message: &UserMessage,
    ) -> Result<(), TurnError> {
        let line = UserLine {
            record_id: RecordId::generate(&*self.clock),
            bot: self.bot.id,
            task: *task,
            device: message.device,
            text: message.text.clone(),
            at_ms: self.clock.now_ms(),
        };
        self.record
            .append(RecordEntry::UserSaid(line))
            .await
            .map(|_| ())
            .map_err(TurnError::Record)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KindCount {
    TurnStarted,
    ModelAsked,
    TurnEnded,
}
