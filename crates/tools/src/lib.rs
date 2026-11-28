pub mod arguments;
pub mod calls;
pub mod edit;
pub mod message_codes;
pub mod offer;
pub mod read;
pub mod registry;
pub mod shell;

pub use arguments::{
    ArgsRejection, EditArgs, ReadArgs, ShellArgs, ToolArgs, args_rejection_word, check_offered,
    parse_call, rejection_word,
};
pub use calls::{
    DEFAULT_SHELL_TIMEOUT_MS, MAX_SHELL_TIMEOUT_MS, build_request, clamp_timeout, subject_of,
};
pub use message_codes::{CODE_TOOL_NOT_IN_TURN, CODE_TOOL_OTHER_COMPUTER};
pub use offer::{COMPUTER_PARAM, ToolDef, offered_tools};
pub use registry::{
    TOOL_EDIT_FILE, TOOL_READ_FILE, TOOL_SHELL, ToolLevel, is_known_tool, level_of, registry,
    tool_list,
};

use anybot_domain::TaskStatus;

use anybot_foundation::{Clock, TimestampMs};
use anybot_protocol::RunEvent;

#[derive(Debug, Clone)]
pub struct ToolOutcome {
    pub status: TaskStatus,
    pub event: RunEvent,
    pub at: TimestampMs,
}

impl ToolOutcome {
    #[must_use]
    pub fn new(status: TaskStatus, event: RunEvent, clock: &dyn Clock) -> Self {
        ToolOutcome {
            status,
            at: clock.now_ms(),
            event,
        }
    }
}
