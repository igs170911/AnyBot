use std::collections::VecDeque;
use std::time::Duration;

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
    ReplyStatus {
        status: u16,
        message: String,
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

#[must_use]
pub fn model_replies_text(text: &str, stream: bool) -> Step {
    Step::ReplyText {
        text: text.to_owned(),
        stream,
    }
}

#[must_use]
pub fn model_think_then_reply(thinking: &str, text: &str, stream: bool) -> Step {
    Step::ThinkThenReply {
        thinking: thinking.to_owned(),
        text: text.to_owned(),
        stream,
    }
}

#[must_use]
pub fn model_calls_tool(name: &str, args: serde_json::Value, stream: bool) -> Step {
    Step::CallTool {
        name: name.to_owned(),
        args,
        stream,
    }
}

#[must_use]
pub fn model_no_response(stream: bool) -> Step {
    Step::NoResponse { stream }
}

#[must_use]
pub fn model_cut_off(text: &str, stream: bool) -> Step {
    Step::CutOffAfter {
        text: text.to_owned(),
        stream,
    }
}

#[must_use]
pub fn model_stall(text: &str, stream: bool) -> Step {
    Step::StallAfter {
        text: text.to_owned(),
        stream,
    }
}

#[must_use]
pub fn model_reply_status(status: u16, message: &str) -> Step {
    Step::ReplyStatus {
        status,
        message: message.to_owned(),
    }
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct RecordedRequest {
    pub path: String,
    pub body: String,
    pub headers: Vec<(String, HeaderValueKind)>,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum HeaderValueKind {
    Present,
    Value(String),
}

const SENSITIVE_HEADERS: [&str; 6] = [
    "authorization",
    "x-api-key",
    "api-key",
    "x-goog-api-key",
    "openai-key",
    "anthropic-key",
];

pub fn record_headers(
    entries: impl Iterator<Item = (String, String)>,
) -> Vec<(String, HeaderValueKind)> {
    let mut recorded = Vec::new();
    for (name, value) in entries {
        let lowered = name.to_ascii_lowercase();
        if SENSITIVE_HEADERS.iter().any(|s| *s == lowered) {
            recorded.push((lowered, HeaderValueKind::Present));
            continue;
        }
        recorded.push((lowered, HeaderValueKind::Value(value)));
    }
    recorded.sort_by(|a, b| a.0.cmp(&b.0));
    recorded
}

pub const CODE_MODEL_SCRIPT_EMPTY: &str = "model-script-empty";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StallDelay {
    pub span: Duration,
}
