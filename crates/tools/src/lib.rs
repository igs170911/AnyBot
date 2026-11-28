pub mod arguments;
pub mod calls;
pub mod edit;
pub mod list;
pub mod message_codes;
pub mod offer;
pub mod read;
pub mod read_result;
pub mod registry;
pub mod result_id;
pub mod search;
pub mod shell;

pub use arguments::{
    ArgsRejection, EditArgs, ListArgs, ReadArgs, ReadResultArgs, SearchArgs, ShellArgs, ToolArgs,
    args_rejection_word, check_offered, parse_call, rejection_word,
};
pub use calls::{
    DEFAULT_SHELL_TIMEOUT_MS, MAX_SHELL_TIMEOUT_MS, build_request, clamp_timeout, subject_of,
};
pub use list::{ListingText, MAX_LIST_ENTRIES, render_listing};
pub use message_codes::{CODE_TOOL_NOT_IN_TURN, CODE_TOOL_OTHER_COMPUTER};
pub use offer::{COMPUTER_PARAM, ToolDef, offered_tools};
pub use read_result::{
    FIND_CONTEXT_CHARS, FIND_MAX_HITS, FindHit, RAW_MAX_BYTES, READ_RESULT_MAX_CHARS,
    clamp_raw_bytes, find_hits, read_result_cut_code, read_result_hit_code,
    read_result_missing_code, read_result_not_found_code, read_result_window_code,
};
pub use registry::{
    TOOL_EDIT_FILE, TOOL_LIST_DIR, TOOL_READ_FILE, TOOL_READ_RESULT, TOOL_SEARCH, TOOL_SHELL,
    ToolLevel, is_known_tool, level_of, registry, tool_list,
};
pub use result_id::result_id;
pub use search::{
    SEARCH_RETURN_LINES, SearchText, render_search, search_cut_code, search_empty_code,
    search_timed_out_code,
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
