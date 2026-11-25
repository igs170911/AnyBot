use proptest::prelude::*;
use proptest::string::string_regex;

use anybot_foundation::{CallId, EventId, SystemClock, TimestampMs};
use anybot_models::{Calibration, Capability, ProviderKind, TokenizerKind, ToolSpec, WireFormat};
use anybot_protocol::{
    BudgetItem, CheckSource, Criterion, DoneWhenChange, KnownTaskEventKind, Proposer, TaskEvent,
    TaskEventKind, ToolEffect,
};

use crate::input::{
    BuildInput, ChatRecord, MemoryRecord, MemorySource, ModelPlan, PinnedSnapshot, StepOutcomeKind,
    StepRecord,
};

pub(crate) fn mixed_text(max_len: usize) -> BoxedStrategy<String> {
    let n = max_len.to_string();
    let ascii = format!("[a-zA-Z0-9 ._,-]{{0,{n}}}");
    let cjk = format!("[\\u{{4e00}}-\\u{{9fff}}]{{0,{n}}}");
    let both = format!("(a-zA-Z0-9 ._,-|[\\u{{4e00}}-\\u{{9fff}}]){{0,{n}}}");
    prop_oneof![
        6 => string_regex(&ascii).unwrap().boxed(),
        3 => string_regex(&cjk).unwrap().boxed(),
        1 => string_regex(&both).unwrap().boxed(),
    ]
    .boxed()
}

pub(crate) fn long_chat_record(prefix: &'static str, index: usize) -> BoxedStrategy<ChatRecord> {
    chat_record(prefix, index)
        .prop_map(move |mut r| {
            r.text = r.text.clone() + &"y".repeat(9_000);
            r
        })
        .boxed()
}

pub(crate) fn chat_record(prefix: &'static str, index: usize) -> BoxedStrategy<ChatRecord> {
    mixed_text(400)
        .prop_map(move |text| ChatRecord {
            record_id: format!("{prefix}-{index}"),
            text,
        })
        .boxed()
}

pub(crate) fn unique_step_refs(steps: Vec<StepRecord>) -> Vec<StepRecord> {
    steps
        .into_iter()
        .enumerate()
        .map(|(index, mut s)| {
            s.tool = format!("tool{index}");
            s.source_ref = format!("ref{index}");
            s
        })
        .collect()
}

pub(crate) fn step_record(index: usize) -> BoxedStrategy<StepRecord> {
    (
        mixed_text(50),
        prop_oneof![
            4 => Just(StepOutcomeKind::Completed),
            2 => Just(StepOutcomeKind::Failed),
            1 => Just(StepOutcomeKind::Unknown),
            1 => Just(StepOutcomeKind::Pending),
        ],
        prop_oneof![
            Just(ToolEffect::ReadOnly),
            Just(ToolEffect::Repeatable),
            Just(ToolEffect::Irreversible)
        ],
        0usize..9_000usize,
        0usize..2usize,
        0usize..4usize,
        prop::bool::ANY,
    )
        .prop_map(
            move |(args, outcome, level, len, exit, errors, unconfirmed)| {
                let content = "x".repeat(len);
                let width = content.chars().count();
                let divisors: [usize; 4] = [2, 3, 4, 5];
                let error_at: Vec<usize> = divisors
                    .iter()
                    .take(errors)
                    .map(|d: &usize| width.checked_div(*d).unwrap_or(0))
                    .collect();
                StepRecord {
                    call_id: CallId::generate(&SystemClock),
                    tool: format!("tool{index}"),
                    args_summary: args,
                    level,
                    unconfirmed,
                    outcome,
                    source_ref: format!("ref{index}"),
                    content,
                    exit_code: if outcome == StepOutcomeKind::Failed {
                        Some(i64::from(i32::try_from(exit).unwrap_or(0)).saturating_add(1))
                    } else {
                        None
                    },
                    error_at,
                }
            },
        )
        .boxed()
}

fn criterion() -> BoxedStrategy<Criterion> {
    prop_oneof![
        3 => mixed_text(30).prop_map(|needle| Criterion::FileContains {
            path: "p.txt".to_owned(),
            needle,
        }),
        1 => mixed_text(300).prop_map(|text| Criterion::ModelJudgement { text }),
    ]
    .boxed()
}

fn known_event() -> BoxedStrategy<TaskEventKind> {
    prop_oneof![
        3 => criterion().prop_map(|c| TaskEventKind::Known(KnownTaskEventKind::DoneWhenChange {
            by: Proposer::User,
            change: DoneWhenChange::SetItems { items: vec![c] },
            before: Vec::new(),
            after: Vec::new(),
            reason: "r".to_owned(),
        })),
        10 => Just(TaskEventKind::Known(KnownTaskEventKind::TurnText { text: "t".to_owned() })),
        6 => (0usize..3usize).prop_map(|i| TaskEventKind::Known(KnownTaskEventKind::ToolIntent {
            call_id: CallId::generate(&SystemClock),
            name: format!("e{i}"),
            args_summary: "a".to_owned(),
            level: ToolEffect::ReadOnly,
        })),
        4 => Just(TaskEventKind::Known(KnownTaskEventKind::EvidenceAdded { summary: "e".to_owned() })),
        2 => Just(TaskEventKind::Known(KnownTaskEventKind::CardShown { action: "q?".to_owned() })),
        2 => Just(TaskEventKind::Known(KnownTaskEventKind::OutcomeUnknown { call_id: CallId::generate(&SystemClock) })),
        1 => Just(TaskEventKind::Known(KnownTaskEventKind::ActionVerified { call_id: CallId::generate(&SystemClock), source: CheckSource::Program })),
        1 => Just(TaskEventKind::Known(KnownTaskEventKind::BudgetUsage { item: BudgetItem::Tokens, delta: 5 })),
        1 => (0usize..3).prop_map(|i| TaskEventKind::Known(KnownTaskEventKind::CriterionPassed { index: i, source: CheckSource::Program })),
        1 => (0usize..3).prop_map(|i| TaskEventKind::Known(KnownTaskEventKind::CriterionFailed { index: i, source: CheckSource::Program, observed: "o".to_owned() })),
        1 => Just(TaskEventKind::Known(KnownTaskEventKind::ModelSwapped { from: "a".to_owned(), to: "b".to_owned(), reason: "r".to_owned() })),
        1 => Just(TaskEventKind::Known(KnownTaskEventKind::ContextBuilt)),
    ]
    .boxed()
}

pub(crate) fn event_with_seq(seq: u64, kind: TaskEventKind) -> TaskEvent {
    TaskEvent {
        event_id: EventId::generate(&SystemClock),
        seq,
        at_ms: TimestampMs::from_millis(seq),
        kind,
    }
}

pub(crate) fn events_strategy(max_len: usize) -> BoxedStrategy<Vec<TaskEvent>> {
    prop::collection::vec(known_event(), 0..max_len)
        .prop_map(move |kinds| {
            let mut out = Vec::new();
            out.push(event_with_seq(
                1,
                TaskEventKind::Known(KnownTaskEventKind::TaskCreated {
                    goal: "g".to_owned(),
                }),
            ));
            for (idx, kind) in kinds.into_iter().enumerate() {
                let seq = u64::try_from(idx).unwrap_or(0).saturating_add(2);
                out.push(event_with_seq(seq, kind.clone()));
                if let TaskEventKind::Known(KnownTaskEventKind::ToolIntent {
                    call_id,
                    name,
                    args_summary,
                    level,
                }) = kind
                {
                    out.push(event_with_seq(
                        seq.saturating_add(1_000),
                        TaskEventKind::Known(KnownTaskEventKind::ToolIntent {
                            call_id,
                            name,
                            args_summary,
                            level,
                        }),
                    ));
                }
            }
            out
        })
        .boxed()
}

pub(crate) fn memory(index: usize) -> BoxedStrategy<MemoryRecord> {
    (0u32..100, prop::bool::ANY)
        .prop_map(move |(relevance, candidate)| MemoryRecord {
            id: format!("m{index}"),
            content: format!("memory {index}"),
            relevance,
            source: MemorySource::ModelInference,
            candidate,
        })
        .boxed()
}

pub(crate) fn model_plan(context_low: u64, context_high: u64) -> BoxedStrategy<ModelPlan> {
    (context_low..context_high, 0u64..1_000)
        .prop_map(|(context, reserved)| ModelPlan {
            capability: Capability {
                provider: ProviderKind::OpenAi,
                model: "m".to_owned(),
                format: WireFormat::OpenAiCompatible,
                context_tokens: context,
                max_output_tokens: 4_000,
                tools: true,
                stream: true,
                thinking: false,
                vision: false,
                cache: true,
                tokenizer: TokenizerKind::Formula,
                source: "t".to_owned(),
            },
            calibration: Calibration::new(),
            reserved_output: reserved,
        })
        .boxed()
}

pub(crate) fn build_input(
    context_low: u64,
    context_high: u64,
    max_steps: usize,
) -> BoxedStrategy<BuildInput> {
    (
        prop::collection::vec(mixed_text(60), 0..6),
        prop::collection::vec(long_chat_record("u", 0), 0..3),
        prop::collection::vec(chat_record("b", 0), 0..3),
        prop::collection::vec(step_record(0), 0..max_steps),
        prop::collection::vec(memory(0), 0..8),
        prop::collection::vec(chat_record("p", 0), 0..12),
        prop::collection::vec(mixed_text(20), 0..4),
        events_strategy(12),
        model_plan(context_low, context_high),
        prop::bool::ANY,
        0usize..3usize,
    )
        .prop_map(
            move |(
                constraints,
                users,
                bots,
                steps,
                memories,
                prior,
                questions,
                events,
                model,
                compact,
                spec_count,
            )| {
                BuildInput {
                    view_limit: crate::numbers::K114_2_DEFAULT_RESULT_CHARS,
                    events,
                    user_turns: users,
                    bot_turns: bots,
                    steps,
                    constraints,
                    open_questions: questions,
                    tool_specs: (0..spec_count).map(tool_spec_at).collect(),
                    system_prompt: "sys".to_owned(),
                    pinned: PinnedSnapshot {
                        version: "v1".to_owned(),
                        items: vec!["pin".to_owned()],
                    },
                    memories,
                    prior_chat: prior,
                    model,
                    compact_requested: compact,
                    secrets: Vec::new(),
                }
            },
        )
        .boxed()
}

pub(crate) fn tool_spec_at(index: usize) -> ToolSpec {
    ToolSpec {
        name: format!("spec{index}"),
        description: format!("spec {index} description"),
        parameters: serde_json::json!({ "type": "object" }),
    }
}
