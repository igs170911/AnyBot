use anybot_domain::{ContextCollapseReason, ContextCollapsed, collapsed_kind};
use anybot_foundation::{CallId, EventId, SystemClock, TimestampMs};
use anybot_models::{Calibration, Capability, ProviderKind, TokenizerKind, ToolSpec, WireFormat};
use anybot_protocol::{KnownTaskEventKind, TaskEvent, TaskEventKind, ToolEffect};

use crate::input::{
    BuildInput, ChatRecord, MemoryRecord, MemorySource, ModelPlan, PinnedSnapshot, StepOutcomeKind,
    StepRecord,
};
use crate::output::{CollapseReason, NewEvent};

fn context_built_kind() -> KnownTaskEventKind {
    KnownTaskEventKind::ContextBuilt {
        manifest_id: "m".to_owned(),
        model: anybot_protocol::ModelName {
            provider: "OpenAi".to_owned(),
            base_url: "http://127.0.0.1:9/v1".to_owned(),
            model: "m".to_owned(),
        },
        estimate_tokens: 1,
    }
}

pub(crate) fn capability(context_tokens: u64) -> Capability {
    Capability {
        provider: ProviderKind::OpenAi,
        model: "test-model".to_owned(),
        format: WireFormat::OpenAiCompatible,
        context_tokens,
        max_output_tokens: 4_000,
        tools: true,
        stream: true,
        thinking: false,
        vision: false,
        cache: true,
        tokenizer: TokenizerKind::Formula,
        source: "test".to_owned(),
    }
}

pub(crate) fn plan(context_tokens: u64) -> ModelPlan {
    ModelPlan {
        capability: capability(context_tokens),
        calibration: Calibration::new(),
        reserved_output: 1_000,
    }
}

pub(crate) fn event(seq: u64, kind: TaskEventKind) -> TaskEvent {
    TaskEvent {
        event_id: EventId::generate(&SystemClock),
        seq,
        at_ms: TimestampMs::from_millis(seq),
        kind,
    }
}

pub(crate) fn known(seq: u64, kind: KnownTaskEventKind) -> TaskEvent {
    event(seq, TaskEventKind::Known(kind))
}

pub(crate) fn record(id: &str, text: &str) -> ChatRecord {
    ChatRecord {
        record_id: id.to_owned(),
        text: text.to_owned(),
        seq: id_tail_number(id),
    }
}

pub(crate) fn id_tail_number(id: &str) -> u64 {
    id.chars()
        .rev()
        .take_while(char::is_ascii_digit)
        .collect::<Vec<char>>()
        .iter()
        .rev()
        .collect::<String>()
        .parse()
        .unwrap_or(0)
}

pub(crate) fn step_len(idx: usize, len: usize, ref_id: &str) -> StepRecord {
    StepRecord {
        seq: u64::try_from(idx).unwrap_or(0).saturating_add(1),
        call_id: CallId::parse(&format!("00000000-0000-0000-0000-{idx:012x}")).unwrap(),
        tool: format!("tool{idx}"),
        args_summary: format!("args{idx}"),
        arguments: serde_json::json!({ "n": idx }),
        level: ToolEffect::ReadOnly,
        unconfirmed: false,
        outcome: StepOutcomeKind::Completed,
        source_ref: ref_id.to_owned(),
        content: {
            let head = format!("s{idx} ");
            let pad = len.saturating_sub(head.chars().count());
            format!("{head}{}", "x".repeat(pad))
        },
        exit_code: None,
        error_at: None,
    }
}

pub(crate) fn base_input() -> BuildInput {
    BuildInput {
        view_limit: crate::numbers::K114_2_DEFAULT_RESULT_CHARS,
        events: Vec::new(),
        user_turns: Vec::new(),
        bot_turns: Vec::new(),
        steps: Vec::new(),
        constraints: Vec::new(),
        open_questions: Vec::new(),
        tool_specs: Vec::new(),
        system_prompt: "sys".to_owned(),
        pinned: PinnedSnapshot {
            version: "v1".to_owned(),
            items: Vec::new(),
        },
        memories: Vec::new(),
        prior_chat: Vec::new(),
        model: plan(100_000),
        compact_requested: false,
    }
}

pub(crate) fn tool_spec(name: &str) -> ToolSpec {
    ToolSpec {
        name: name.to_owned(),
        description: format!("{name} description"),
        parameters: serde_json::json!({ "type": "object" }),
    }
}

pub(crate) fn memory(id: &str, relevance: u32, candidate: bool) -> MemoryRecord {
    MemoryRecord {
        id: id.to_owned(),
        content: format!("memory {id}"),
        relevance,
        source: MemorySource::UserSaid,
        candidate,
    }
}

pub(crate) fn push_new(events: &mut Vec<TaskEvent>, new: &[NewEvent]) {
    for (offset, e) in new.iter().enumerate() {
        let seq = u64::try_from(events.len()).unwrap_or(0).saturating_add(1);
        let seq = seq.saturating_add(u64::try_from(offset).unwrap_or(0));
        match e {
            NewEvent::ContextBuilt => events.push(known(seq, context_built_kind())),
            NewEvent::MemoryDropped { ids } => {
                events.push(event(seq, anybot_domain::memory_dropped_kind(ids)));
            }
            NewEvent::Collapsed { ids, reason } => {
                let domain = match reason {
                    CollapseReason::Accumulated => ContextCollapseReason::Accumulated,
                    CollapseReason::Budget => ContextCollapseReason::Budget,
                    CollapseReason::Compact => ContextCollapseReason::Compact,
                    CollapseReason::ModelSwapped => ContextCollapseReason::ModelSwapped,
                    CollapseReason::ReplaceAll => ContextCollapseReason::ReplaceAll,
                };
                events.push(event(
                    seq,
                    collapsed_kind(&ContextCollapsed {
                        ids: ids.clone(),
                        reason: domain,
                    }),
                ));
            }
        }
    }
}
