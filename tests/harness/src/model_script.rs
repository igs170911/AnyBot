use std::collections::VecDeque;

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct UsageScript {
    pub input_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_write_tokens: u64,
    pub output_tokens: u64,
    pub thinking_tokens: u64,
}

impl UsageScript {
    #[must_use]
    pub fn plain(input_tokens: u64, output_tokens: u64) -> Self {
        Self {
            input_tokens,
            cache_read_tokens: 0,
            cache_write_tokens: 0,
            output_tokens,
            thinking_tokens: 0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ToolCallScript {
    pub name: String,
    pub arguments: String,
    pub id: Option<String>,
    pub argument_parts: Vec<String>,
}

impl ToolCallScript {
    #[must_use]
    pub fn named(name: &str, arguments: &str) -> Self {
        Self {
            name: name.to_owned(),
            arguments: arguments.to_owned(),
            id: Some("call_fake".to_owned()),
            argument_parts: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Step {
    ReplyText {
        text: String,
        stream: bool,
    },
    ThinkThenReply {
        thinking: String,
        text: String,
        stream: bool,
    },
    CallTool {
        name: String,
        args: serde_json::Value,
        stream: bool,
    },
    NoResponse {
        stream: bool,
    },
    CutOffAfter {
        text: String,
        stream: bool,
    },
    StallAfter {
        text: String,
        stream: bool,
    },
    HoldThenFinish {
        text: String,
        stream: bool,
    },
    ReplyStatus {
        status: u16,
        message: String,
        retry_after_secs: Option<u64>,
        body_kind: ErrorBodyKind,
    },
    StreamError {
        busy: bool,
        message: String,
    },
    EmptyBody,
    Redirect {
        location: String,
    },
    ModelList {
        ids: Vec<String>,
        silent: bool,
    },
    DelayHeaders {
        seconds: u64,
        inner: Box<Step>,
    },
    AcceptThenDelay {
        seconds: u64,
        inner: Box<Step>,
    },
    UsageReply {
        text: String,
        usage: UsageScript,
        stream: bool,
        omit_usage: bool,
    },
    ThinkingBlock {
        thinking: String,
        signature: String,
        text: String,
        redacted: bool,
        stream: bool,
    },
    StopReason {
        text: String,
        stop_reason: String,
        stream: bool,
    },
    ToolCalls {
        calls: Vec<ToolCallScript>,
        finish_reason: String,
        stream: bool,
        cut_mid_arguments: bool,
        length_stop: bool,
    },
    ThinkThenTool {
        thinking: String,
        signature: String,
        calls: Vec<ToolCallScript>,
        stream: bool,
    },
    ToolCallsWithUsage {
        calls: Vec<ToolCallScript>,
        usage: UsageScript,
        stream: bool,
    },
}

#[derive(Default)]
pub struct Script {
    steps: VecDeque<Step>,
}

impl Script {
    pub fn push(&mut self, step: Step) {
        self.steps.push_back(step);
    }

    pub fn take(&mut self) -> Option<Step> {
        self.steps.pop_front()
    }

    pub fn clear(&mut self) {
        self.steps.clear();
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.steps.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }
}

#[path = "model_keys.rs"]
mod model_keys;
#[path = "model_steps.rs"]
mod model_steps;

pub use model_keys::{
    CODE_MODEL_SCRIPT_EMPTY, HeaderValueKind, KeyMatch, RecordedRequest, StallDelay, classify_key,
    presented_key, record_headers,
};
pub use model_steps::model_hold_then_finish;
pub use model_steps::{
    ErrorBodyKind, model_accept_then_delay, model_calls_tool, model_cut_off, model_delay_headers,
    model_empty_body, model_error_body, model_list, model_list_silent, model_no_response,
    model_omit_usage, model_redacted_thinking, model_redirect, model_replies_text,
    model_reply_status, model_reply_status_retry, model_stall, model_stop_reason,
    model_stream_error, model_think_then_reply, model_think_then_tool, model_thinking_block,
    model_tool_calls, model_tool_calls_cut, model_tool_calls_length, model_tool_calls_with_usage,
    model_usage, random_reply,
};
