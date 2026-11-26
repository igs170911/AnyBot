use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use anybot_protocol::{
    BudgetItem, Criterion, DoneWhenChange, Proposer, TaskDetail, TaskStatus, ToolEffect,
    effect_level,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CriterionItem {
    pub def: Criterion,
    pub passed: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingProposal {
    pub by: Proposer,
    pub change: DoneWhenChange,
    pub before: Vec<Criterion>,
    pub after: Vec<Criterion>,
    pub reason: String,
}

pub const DEFAULT_STEPS_LIMIT: u64 = 300;
pub const DEFAULT_TOOL_CALLS_LIMIT: u64 = 600;
pub const DEFAULT_ELAPSED_SECS_LIMIT: u64 = 4 * 3600;
pub const TURN_STEP_GUARD: u64 = 50_000;
pub const UNLIMITED: u64 = u64::MAX;
pub const ROUND_ITEMS: [BudgetItem; 3] = [
    BudgetItem::Steps,
    BudgetItem::ToolCalls,
    BudgetItem::Elapsed,
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BudgetOver {
    pub item: BudgetItem,
    pub used: u64,
    pub limit: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BudgetState {
    #[serde(default = "default_limits")]
    pub limits: BTreeMap<BudgetItem, u64>,
    #[serde(default)]
    pub usage: BTreeMap<BudgetItem, u64>,
    #[serde(default)]
    pub explicit: BTreeSet<BudgetItem>,
    #[serde(default)]
    pub over: Option<BudgetOver>,
    #[serde(default)]
    pub round_baseline: BTreeMap<BudgetItem, u64>,
    #[serde(default)]
    pub turn_steps: u64,
    #[serde(default)]
    pub step_fails: BTreeMap<String, u64>,
    #[serde(default)]
    pub turn_start_ms: Option<u64>,
    #[serde(default)]
    pub turn_paused_ms: u64,
    #[serde(default)]
    pub card_start_ms: Option<u64>,
    #[serde(default)]
    pub last_ms: u64,
}

fn default_limits() -> BTreeMap<BudgetItem, u64> {
    let mut limits = BTreeMap::new();
    limits.insert(BudgetItem::Steps, DEFAULT_STEPS_LIMIT);
    limits.insert(BudgetItem::ToolCalls, DEFAULT_TOOL_CALLS_LIMIT);
    limits.insert(BudgetItem::Elapsed, DEFAULT_ELAPSED_SECS_LIMIT);
    limits
}

impl BudgetState {
    #[must_use]
    pub fn new() -> Self {
        BudgetState {
            limits: default_limits(),
            usage: BTreeMap::new(),
            explicit: BTreeSet::new(),
            over: None,
            round_baseline: BTreeMap::new(),
            turn_steps: 0,
            step_fails: BTreeMap::new(),
            turn_start_ms: None,
            turn_paused_ms: 0,
            card_start_ms: None,
            last_ms: 0,
        }
    }

    pub fn add_usage(&mut self, item: BudgetItem, delta: u64) {
        let current = self.usage.get(&item).copied().unwrap_or(0);
        let next = current.saturating_add(delta);
        self.usage.insert(item, next);
    }

    #[must_use]
    pub fn used(&self, item: BudgetItem) -> u64 {
        self.usage.get(&item).copied().unwrap_or(0)
    }

    #[must_use]
    pub fn step_fail_limit(&self) -> u64 {
        self.limits
            .get(&BudgetItem::Retries)
            .copied()
            .unwrap_or(UNLIMITED)
    }
}

impl Default for BudgetState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateFlags {
    pub step_rejected: bool,
    pub step_failed: bool,
    pub acceptance_unavailable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceFlags {
    pub actions_taken: bool,
    pub claims_ok: bool,
    pub turn_reply: bool,
    pub delivered_reported: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportFlags {
    pub user_confirmed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskView {
    pub status: TaskStatus,
    pub detail: Option<TaskDetail>,
    pub goal: String,
    pub criteria: Vec<CriterionItem>,
    pub pending_proposals: Vec<PendingProposal>,
    pub budget: BudgetState,
    pub action_levels: BTreeMap<String, ToolEffect>,
    pub pending_actions: BTreeSet<String>,
    pub verified_actions: BTreeSet<String>,
    pub unresolved_unknowns: BTreeSet<String>,
    pub gate: GateFlags,
    pub evidence: EvidenceFlags,
    pub report: ReportFlags,
    #[serde(default)]
    pub goal_task: bool,
}

#[must_use]
pub fn initial_view() -> TaskView {
    TaskView {
        status: TaskStatus::Running,
        detail: Some(TaskDetail::Queued),
        goal: String::new(),
        criteria: Vec::new(),
        pending_proposals: Vec::new(),
        budget: BudgetState::new(),
        action_levels: BTreeMap::new(),
        pending_actions: BTreeSet::new(),
        verified_actions: BTreeSet::new(),
        unresolved_unknowns: BTreeSet::new(),
        gate: GateFlags {
            step_rejected: false,
            step_failed: false,
            acceptance_unavailable: false,
        },
        evidence: EvidenceFlags {
            actions_taken: false,
            claims_ok: true,
            turn_reply: false,
            delivered_reported: false,
        },
        report: ReportFlags {
            user_confirmed: false,
        },
        goal_task: false,
    }
}

pub fn mark_goal_task(view: &mut TaskView) {
    view.goal_task = true;
}

impl TaskView {
    pub fn note_action(&mut self, call: &str) {
        let level = self
            .action_levels
            .get(call)
            .copied()
            .unwrap_or_else(|| effect_level(None));
        if level != ToolEffect::ReadOnly {
            self.pending_actions.insert(call.to_owned());
        }
    }

    #[must_use]
    pub fn unverified_actions(&self) -> bool {
        self.pending_actions
            .iter()
            .any(|call| !self.verified_actions.contains(call))
    }

    #[must_use]
    pub fn criteria_all_passed(&self) -> bool {
        self.criteria.iter().all(|item| item.passed == Some(true))
    }

    #[must_use]
    pub fn completion_ready(&self) -> bool {
        self.criteria_all_passed()
            && self.evidence.claims_ok
            && self.unresolved_unknowns.is_empty()
            && self.budget.over.is_none()
            && !self.gate.acceptance_unavailable
    }

    pub fn set_running(&mut self, detail: TaskDetail) {
        self.status = TaskStatus::Running;
        self.detail = Some(detail);
    }

    pub fn complete(&mut self) {
        self.status = TaskStatus::Completed;
        self.detail = Some(self.completed_detail());
    }

    #[must_use]
    pub fn completed_detail(&self) -> TaskDetail {
        if self.report.user_confirmed {
            return TaskDetail::UserConfirmed;
        }
        if self.evidence.delivered_reported {
            return TaskDetail::Delivered;
        }
        if self.evidence.turn_reply && !self.evidence.actions_taken {
            return TaskDetail::Replied;
        }
        TaskDetail::Accepted
    }

    pub fn upgrade_completed_detail(&mut self) {
        if self.status == TaskStatus::Completed {
            self.detail = Some(self.completed_detail());
        }
    }
}

#[cfg(test)]
#[path = "task_view_tests.rs"]
mod tests;
