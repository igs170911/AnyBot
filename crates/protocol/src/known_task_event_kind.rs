use anybot_foundation::CallId;
use serde::{Deserialize, Serialize};

use crate::{
    BudgetItem, CardAnswer, CheckSource, Criterion, DeviceReportKind, DoneWhenChange, Proposer,
    ReconcileOutcome, ToolEffect,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema, utoipa::ToSchema))]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
pub enum KnownTaskEventKind {
    TaskCreated {
        goal: String,
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
    TurnStarted,
    TurnEnded {
        reply_delivered: bool,
    },
    TurnText {
        text: String,
    },
    ContextBuilt,
    ModelSwapped {
        from: String,
        to: String,
        reason: String,
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
    },
    EvidenceAdded {
        summary: String,
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
    ClaimChecked {
        covers_claim: bool,
        verified: bool,
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
    },
    CardAnswered {
        answer: CardAnswer,
    },
    DeviceReport {
        kind: DeviceReportKind,
        file: String,
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
