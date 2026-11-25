use serde_json as _;

#[cfg(test)]
use serde_json as _;
pub mod goal_brake;
pub mod goal_followup;
pub mod task_attention;
pub mod task_budget;
pub mod task_context_events;
pub mod task_effects;
pub mod task_engine;
pub mod task_engine_events;
pub mod task_engine_evidence;
pub mod task_engine_lifecycle;
pub mod task_entry;
pub mod task_gate;
pub mod task_journal;
pub mod task_replay;
pub mod task_stall;
pub mod task_view;

pub use anybot_protocol::TaskStatus;
pub use task_attention::{counts_as_failure, counts_in_task_badge};
pub use task_context_events::{
    CONTEXT_COLLAPSED_KIND, ContextCollapseReason, ContextCollapsed, collapsed_kind,
    parse_collapsed,
};
pub use task_effects::{ResumeAction, resume_action};
pub use task_engine::advance;
pub use task_entry::{EntryInput, EntryTracker};
pub use task_gate::finish_turn;
pub use task_journal::{JournalOp, TaskJournal};
pub use task_replay::{
    ReplayGap, SNAPSHOT_FORMAT_VERSION, TaskSnapshot, missing_seqs, replay, replay_from,
    snapshot_at,
};
pub use task_stall::{RunningSpan, is_stalled};
pub use task_view::{TaskView, initial_view};

#[must_use]
pub fn needs_attention(status: TaskStatus) -> bool {
    matches!(
        status,
        TaskStatus::AwaitingUser | TaskStatus::Stuck | TaskStatus::Failed
    )
}
