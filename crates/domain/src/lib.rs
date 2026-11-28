use serde_json as _;

#[cfg(test)]
use serde_json as _;
pub mod account_name;
pub mod card_waiting;
pub mod computer_actions;
pub mod computer_name;
pub mod computer_presence;
pub mod conn_auth;
pub mod goal_brake;
pub mod goal_followup;
pub mod invite;
pub mod mailbox_letter;
pub mod mailbox_route;
pub mod memory_admit;
pub mod memory_projection;
mod memory_projection_ops;
pub mod memory_scan;
pub mod memory_secret_rules;
pub mod memory_types;
pub mod pairing_rules;
pub mod record_actions;
pub mod record_rules;
pub mod referee;
pub mod session_term;
pub mod settings_merge;
pub mod settings_registry;
pub mod source_addr;
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
mod task_lifecycle_ops;
pub mod task_replay;
pub mod task_stall;
pub mod task_view;
pub mod thumbs;
#[cfg(test)]
mod thumbs_tests;
pub mod turn_summary;
#[cfg(test)]
mod turn_summary_tests;
pub mod two_step;
pub mod version_release;
#[cfg(test)]
mod version_release_tests;
pub mod window;

#[cfg(test)]
mod card_waiting_tests;
#[cfg(test)]
mod conn_auth_tests;
#[cfg(test)]
mod memory_prop_tests;
#[cfg(test)]
mod memory_tests;
#[cfg(test)]
mod memory_tests_extra;
#[cfg(test)]
#[cfg(test)]
mod record_actions_tests;
#[cfg(test)]
mod record_rules_tests;
#[cfg(test)]
mod referee_tests;
#[cfg(test)]
mod settings_merge_lists_tests;
#[cfg(test)]
mod settings_merge_proptest;
#[cfg(test)]
pub mod settings_merge_shared;
#[cfg(test)]
mod settings_merge_tests;
#[cfg(test)]
mod settings_registry_tests;

pub use anybot_foundation::{DeviceId, TimestampMs};
pub use anybot_protocol::TaskStatus;
pub use invite::{INVITE_ALPHABET, alphabet_len, format_invite, is_valid_invite};
pub use memory_admit::{
    Admission, AdmitReject, AdmittedMemory, CODE_MEMORY_CONTRADICTION_MISSING,
    CODE_MEMORY_EMPTY_CONTENT, MemoryClaimSource, MemoryProposal, admit,
};
pub use memory_projection::project;
pub use memory_scan::{
    CODE_MEMORY_SCAN_INJECTION, CODE_MEMORY_SCAN_INVISIBLE, CODE_MEMORY_SCAN_SECRET, ScanCategory,
    ScanFinding, scan,
};
pub use memory_secret_rules::{
    SecretMatch, find_secret_patterns, has_secret_pattern, redact_secret_patterns,
};
pub use memory_types::{
    ContradictionNote, MAX_MEMORY_CHARS, MemoryBook, MemoryEventContent, MemoryEventKind,
    MemoryEventRecord, MemoryEvidenceRef, MemoryKind, MemoryOrigin, MemoryRecordError, MemoryScope,
    MemorySource, MemoryState, MemoryVersionEntry, ProjectedMemory, normalize_for_match,
    truncate_memory_content,
};
pub use record_rules::{
    AcceptContext, AcceptOutcome, BotState, DedupFields, decide_accept, dedup_same,
    is_device_forbidden,
};
pub use settings_merge::{
    CorrectedTimeError, FieldEdit, FieldKey, SettingsState, agreed, corrected_time,
};
pub use task_attention::{counts_as_failure, counts_in_task_badge};
pub use task_context_events::{
    CONTEXT_COLLAPSED_KIND, CONTEXT_MEMORY_DROPPED_KIND, ContextCollapseReason, ContextCollapsed,
    collapsed_kind, memory_dropped_kind, parse_collapsed, parse_memory_dropped,
};
pub use task_effects::{ResumeAction, resume_action};
pub use task_engine::advance;
pub use task_entry::{EntryInput, EntryTracker};
pub use task_gate::{end_turn, finish_turn};
pub use task_journal::{JournalOp, TaskJournal};
pub use task_replay::{
    ReplayGap, SNAPSHOT_FORMAT_VERSION, TaskSnapshot, missing_seqs, replay, replay_from,
    snapshot_at,
};
pub use task_stall::{RunningSpan, is_stalled};
pub use task_view::{TaskView, initial_view};
pub use turn_summary::{SummaryError, TurnSummary, turn_summary};

#[must_use]
pub fn needs_attention(status: TaskStatus) -> bool {
    matches!(
        status,
        TaskStatus::AwaitingUser | TaskStatus::Stuck | TaskStatus::Failed
    )
}
