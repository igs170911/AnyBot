use serde_json::Value;

pub const ACCOUNT_STREAM: &str = "account";

#[must_use]
pub fn bot_stream(bot_id: &str) -> String {
    format!("bot/{bot_id}")
}

#[must_use]
pub fn written_by_server(kind: &str) -> bool {
    anybot_domain::record_rules::DEVICE_FORBIDDEN_KINDS.contains(&kind)
}

#[derive(Debug, Clone, PartialEq)]
pub struct LocalRecord {
    pub stream: String,
    pub seq: i64,
    pub record_id: String,
    pub kind: String,
    pub content: Value,
    pub created_at_ms: i64,
    pub offline_sent: bool,
    pub task_id: Option<String>,
    pub task_seq: Option<i64>,
    pub purged: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Progress {
    pub seq: i64,
    pub record_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApplyOutcome {
    pub inserted: u64,
    pub through_seq: i64,
    pub gap: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutboxState {
    Pending,
    Rejected,
}

impl OutboxState {
    #[must_use]
    pub fn as_text(self) -> &'static str {
        match self {
            OutboxState::Pending => "pending",
            OutboxState::Rejected => "rejected",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct NewOutbox {
    pub stream: String,
    pub record_id: String,
    pub kind: String,
    pub content: Value,
    pub created_at_ms: i64,
    pub task_id: Option<String>,
    pub offline_at_create: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OutboxRow {
    pub local_id: i64,
    pub stream: String,
    pub record_id: String,
    pub kind: String,
    pub content: Value,
    pub created_at_ms: i64,
    pub task_id: Option<String>,
    pub offline_at_create: bool,
    pub attempted_at_ms: Option<i64>,
    pub offline_sent: Option<bool>,
    pub state: OutboxState,
    pub reject_code: Option<String>,
    pub rejected_at_ms: Option<i64>,
}
