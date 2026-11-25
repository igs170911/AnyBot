use super::{Step, ToolCallScript, UsageScript};

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
pub fn model_hold_then_finish(text: &str, stream: bool) -> Step {
    Step::HoldThenFinish {
        text: text.to_owned(),
        stream,
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ErrorBodyKind {
    #[default]
    OpenAi,
    Anthropic,
    Vllm,
}

#[must_use]
pub fn model_reply_status(status: u16, message: &str) -> Step {
    Step::ReplyStatus {
        status,
        message: message.to_owned(),
        retry_after_secs: None,
        body_kind: ErrorBodyKind::OpenAi,
    }
}

#[must_use]
pub fn model_reply_status_retry(status: u16, message: &str, retry_after_secs: u64) -> Step {
    Step::ReplyStatus {
        status,
        message: message.to_owned(),
        retry_after_secs: Some(retry_after_secs),
        body_kind: ErrorBodyKind::OpenAi,
    }
}

#[must_use]
pub fn model_error_body(status: u16, message: &str, body_kind: ErrorBodyKind) -> Step {
    Step::ReplyStatus {
        status,
        message: message.to_owned(),
        retry_after_secs: None,
        body_kind,
    }
}

#[must_use]
pub fn model_stream_error(busy: bool, message: &str) -> Step {
    Step::StreamError {
        busy,
        message: message.to_owned(),
    }
}

#[must_use]
pub fn model_empty_body() -> Step {
    Step::EmptyBody
}

#[must_use]
pub fn model_redirect(location: &str) -> Step {
    Step::Redirect {
        location: location.to_owned(),
    }
}

#[must_use]
pub fn model_list(ids: Vec<String>) -> Step {
    Step::ModelList { ids, silent: false }
}

#[must_use]
pub fn model_list_silent() -> Step {
    Step::ModelList {
        ids: Vec::new(),
        silent: true,
    }
}

#[must_use]
pub fn model_usage(text: &str, usage: UsageScript, stream: bool) -> Step {
    Step::UsageReply {
        text: text.to_owned(),
        usage,
        stream,
        omit_usage: false,
    }
}

#[must_use]
pub fn model_omit_usage(text: &str) -> Step {
    Step::UsageReply {
        text: text.to_owned(),
        usage: UsageScript::plain(0, 0),
        stream: true,
        omit_usage: true,
    }
}

#[must_use]
pub fn model_thinking_block(thinking: &str, signature: &str, text: &str, stream: bool) -> Step {
    Step::ThinkingBlock {
        thinking: thinking.to_owned(),
        signature: signature.to_owned(),
        text: text.to_owned(),
        redacted: false,
        stream,
    }
}

#[must_use]
pub fn model_redacted_thinking(data: &str, text: &str, stream: bool) -> Step {
    Step::ThinkingBlock {
        thinking: String::new(),
        signature: data.to_owned(),
        text: text.to_owned(),
        redacted: true,
        stream,
    }
}

#[must_use]
pub fn model_stop_reason(text: &str, stop_reason: &str, stream: bool) -> Step {
    Step::StopReason {
        text: text.to_owned(),
        stop_reason: stop_reason.to_owned(),
        stream,
    }
}

#[must_use]
pub fn model_tool_calls(calls: Vec<ToolCallScript>, finish_reason: &str, stream: bool) -> Step {
    tool_calls_step(calls, finish_reason, stream, false, false)
}

#[must_use]
pub fn model_tool_calls_cut(calls: Vec<ToolCallScript>, stream: bool) -> Step {
    tool_calls_step(calls, "tool_calls", stream, true, false)
}

#[must_use]
pub fn model_tool_calls_length(calls: Vec<ToolCallScript>, stream: bool) -> Step {
    tool_calls_step(calls, "length", stream, false, true)
}

#[must_use]
pub fn model_think_then_tool(
    thinking: &str,
    signature: &str,
    calls: Vec<ToolCallScript>,
    stream: bool,
) -> Step {
    Step::ThinkThenTool {
        thinking: thinking.to_owned(),
        signature: signature.to_owned(),
        calls,
        stream,
    }
}

#[must_use]
pub fn random_reply(seed: u64) -> (Step, bool, bool) {
    let complete = !seed.is_multiple_of(5);
    let has_tool = seed.is_multiple_of(2);
    let stream = !seed.is_multiple_of(3);
    let call = ToolCallScript {
        name: "read_file".to_owned(),
        arguments: r#"{"path":"a.txt"}"#.to_owned(),
        id: Some(format!("call_{seed}")),
        argument_parts: vec![r#"{"path":"#.to_owned(), r#""a.txt"}"#.to_owned()],
    };
    let step = if !complete {
        model_tool_calls_cut(vec![call], stream)
    } else if has_tool {
        let reason = if seed.is_multiple_of(7) {
            "stop"
        } else {
            "tool_calls"
        };
        model_tool_calls(vec![call], reason, stream)
    } else {
        model_think_then_reply("plan", "ok", stream)
    };
    (step, complete && has_tool, stream)
}

fn tool_calls_step(
    calls: Vec<ToolCallScript>,
    finish_reason: &str,
    stream: bool,
    cut_mid_arguments: bool,
    length_stop: bool,
) -> Step {
    Step::ToolCalls {
        calls,
        finish_reason: finish_reason.to_owned(),
        stream,
        cut_mid_arguments,
        length_stop,
    }
}
