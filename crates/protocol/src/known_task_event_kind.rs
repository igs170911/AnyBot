use anybot_foundation::{BotId, CallId, TaskId};
use serde::{Deserialize, Serialize};

use crate::{
    AnswerOrigin, BudgetItem, CardAnswer, CardButton, CardKind, CheckSource, ComputerRef,
    Criterion, DeviceReportKind, DoneWhenChange, EvidenceRecord, FinishReason, GateRecord,
    ModelName, NativeRef, NotRunReason, Proposer, QuestionAnswer, QuestionItem, ReasonPayload,
    ReconcileOutcome, ResumeCard, TaskOpener, ThinkingChoice, TokenUsage, ToolEffect, ToolRequest,
    TurnEndReason, TurnInitiator,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
pub enum KnownTaskEventKind {
    TaskCreated {
        goal: String,
        opened_by: TaskOpener,
        #[cfg_attr(feature = "schema", schema(value_type = String, format = "uuid"))]
        bot: BotId,
        #[cfg_attr(feature = "schema", schema(value_type = Option<String>, format = "uuid"))]
        parent: Option<TaskId>,
    },
    DoneWhenChange {
        by: Proposer,
        change: DoneWhenChange,
        before: Vec<Criterion>,
        after: Vec<Criterion>,
        reason: String,
    },
    DoneWhenAppliedByUser {
        change: DoneWhenChange,
    },
    TaskCancelled {
        reason: String,
    },
    TaskStopped,
    Wake {
        reason: String,
    },
    TurnStarted {
        turn: u32,
        initiator: TurnInitiator,
        computer: ComputerRef,
        model: ModelName,
        thinking: ThinkingChoice,
    },
    TurnEnded {
        reason: TurnEndReason,
        reply_delivered: bool,
    },
    ContextBuilt {
        manifest_id: String,
        model: ModelName,
        estimate_tokens: u64,
    },
    ModelAsked {
        step: u32,
    },
    ModelAnswer {
        text: String,
        thinking: Option<String>,
        tool_requests: Vec<ToolRequest>,
        finish: FinishReason,
        native: Option<NativeRef>,
    },
    ModelUsage {
        usage: Option<TokenUsage>,
        manifest_id: String,
    },
    ToolNotRun {
        #[cfg_attr(feature = "schema", schema(value_type = String, format = "uuid"))]
        call_id: CallId,
        reason: NotRunReason,
    },
    ModelSwapped {
        from: String,
        to: String,
        reason: String,
    },
    GateResult {
        #[cfg_attr(feature = "schema", schema(value_type = String, format = "uuid"))]
        call_id: CallId,
        gate: GateRecord,
    },
    ToolIntent {
        #[cfg_attr(feature = "schema", schema(value_type = String, format = "uuid"))]
        call_id: CallId,
        name: String,
        args_summary: String,
        level: ToolEffect,
    },
    ToolStarted {
        #[cfg_attr(feature = "schema", schema(value_type = String, format = "uuid"))]
        call_id: CallId,
    },
    ToolCompleted {
        #[cfg_attr(feature = "schema", schema(value_type = String, format = "uuid"))]
        call_id: CallId,
        summary: String,
        raw_id: Option<String>,
    },
    ToolFailed {
        #[cfg_attr(feature = "schema", schema(value_type = String, format = "uuid"))]
        call_id: CallId,
        name: String,
        args_summary: String,
        error: String,
    },
    OutcomeUnknown {
        #[cfg_attr(feature = "schema", schema(value_type = String, format = "uuid"))]
        call_id: CallId,
    },
    Reconciled {
        #[cfg_attr(feature = "schema", schema(value_type = String, format = "uuid"))]
        call_id: CallId,
        outcome: ReconcileOutcome,
    },
    ActionVerified {
        #[cfg_attr(feature = "schema", schema(value_type = String, format = "uuid"))]
        call_id: CallId,
        source: CheckSource,
        #[cfg_attr(feature = "schema", schema(value_type = Vec<String>, format = "uuid"))]
        evidence_ids: Vec<anybot_foundation::EvidenceId>,
    },
    EvidenceAdded {
        record: Box<EvidenceRecord>,
    },
    CriterionPassed {
        index: usize,
        source: CheckSource,
    },
    CriterionFailed {
        index: usize,
        source: CheckSource,
        observed: String,
    },
    AcceptanceUnavailable,
    VerificationStarted,
    ClaimChecked {
        covers_claim: bool,
        verified: bool,
        reasons: Vec<ReasonPayload>,
    },
    BudgetUsage {
        item: BudgetItem,
        delta: u64,
    },
    BudgetLimit {
        item: BudgetItem,
        limit: u64,
    },
    CardShown {
        action: String,
        card: String,
        #[serde(default)]
        kind: CardKind,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        resume: Option<ResumeCard>,
        buttons: Vec<CardButton>,
        reason: ReasonPayload,
        always_label: Option<ReasonPayload>,
        notes: Vec<ReasonPayload>,
        classes: Vec<String>,
    },
    CardAnswered {
        answer: CardAnswer,
        #[serde(default)]
        origin: AnswerOrigin,
    },
    QuestionShown {
        card: String,
        questions: Vec<QuestionItem>,
    },
    QuestionAnswered {
        card: String,
        answer: QuestionAnswer,
    },
    DeviceReport {
        kind: DeviceReportKind,
        file: String,
    },
    DeliveryReport {
        report: Box<crate::turn_records::DeliveryReportRecord>,
    },
    ModelUnreachable,
    ComputerOffline,
    GoalPaused {
        reason: String,
    },
    Stalled {
        minutes: u32,
    },
}
