use proptest::prelude::*;

pub(crate) use crate::fixtures_extra::{memory, model_plan, tool_spec_at};
use anybot_foundation::{CallId, SystemClock};
use anybot_protocol::{Criterion, KnownTaskEventKind, TaskEvent, TaskEventKind, ToolEffect};

use crate::input::{BuildInput, ChatRecord, PinnedSnapshot, StepOutcomeKind, StepRecord};

fn ascii_char() -> impl Strategy<Value = char> {
    prop::char::ranges(
        vec![
            'a'..='z',
            'A'..='Z',
            '0'..='9',
            ' '..=' ',
            '.'..='.',
            '_'..='_',
            ','..=',',
            '-'..='-',
        ]
        .into(),
    )
}

fn cjk_char() -> impl Strategy<Value = char> {
    prop::char::range('\u{4e00}', '\u{9fff}')
}

pub(crate) fn mixed_text(max_len: usize) -> BoxedStrategy<String> {
    prop_oneof![
        6 => prop::collection::vec(ascii_char(), 0..=max_len),
        3 => prop::collection::vec(cjk_char(), 0..=max_len),
        1 => prop::collection::vec(prop_oneof![ascii_char(), cjk_char()], 0..=max_len),
    ]
    .prop_map(|chars| chars.into_iter().collect())
    .boxed()
}

pub(crate) fn long_chat_record(prefix: &'static str, index: usize) -> BoxedStrategy<ChatRecord> {
    chat_record(prefix, index)
        .prop_map(move |mut r| {
            r.text = r.text.clone() + &"y".repeat(240);
            r
        })
        .boxed()
}

pub(crate) fn chat_record(prefix: &'static str, index: usize) -> BoxedStrategy<ChatRecord> {
    (0u64..400, mixed_text(400))
        .prop_map(move |(extra, text)| ChatRecord {
            seq: u64::try_from(index)
                .unwrap_or(0)
                .saturating_add(1)
                .saturating_add(extra),
            record_id: format!("{prefix}-{index}"),
            text,
        })
        .boxed()
}

pub(crate) fn renumbered(records: Vec<ChatRecord>) -> Vec<ChatRecord> {
    records
        .into_iter()
        .enumerate()
        .map(|(index, mut r)| {
            r.seq = u64::try_from(index).unwrap_or(0).saturating_add(1);
            r.record_id = format!("{}-{index}", r.record_id);
            r
        })
        .collect()
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

pub(crate) fn unique_memory_ids(
    memories: Vec<crate::input::MemoryRecord>,
) -> Vec<crate::input::MemoryRecord> {
    memories
        .into_iter()
        .enumerate()
        .map(|(index, mut m)| {
            m.id = format!("m{index}");
            m
        })
        .collect()
}

pub(crate) fn step_record(index: usize) -> BoxedStrategy<StepRecord> {
    (
        mixed_text(50),
        prop_oneof![
            Just(StepOutcomeKind::Completed),
            Just(StepOutcomeKind::Failed),
            Just(StepOutcomeKind::Unknown),
            Just(StepOutcomeKind::Pending)
        ],
        prop_oneof![
            Just(ToolEffect::ReadOnly),
            Just(ToolEffect::Repeatable),
            Just(ToolEffect::Irreversible)
        ],
        mixed_text(1_600),
        0usize..2usize,
        prop::bool::ANY,
    )
        .prop_map(move |(args, outcome, level, content, exit, unconfirmed)| {
            let exit_code = if outcome == StepOutcomeKind::Failed {
                Some(i64::from(i32::try_from(exit).unwrap_or(0)).saturating_add(1))
            } else {
                None
            };
            let error_at = crate::error_line::find_error_line(&content, exit_code);
            StepRecord {
                seq: u64::try_from(index).unwrap_or(0).saturating_add(1),
                call_id: CallId::generate(&SystemClock),
                tool: format!("tool{index}"),
                args_summary: args.clone(),
                arguments: serde_json::json!({ "path": args }),
                level,
                unconfirmed,
                outcome,
                source_ref: format!("ref{index}"),
                content,
                exit_code,
                error_at,
            }
        })
        .boxed()
}

pub(crate) fn criterion() -> BoxedStrategy<Criterion> {
    prop_oneof![
        3 => mixed_text(30).prop_map(|needle| Criterion::FileContains {
            path: "p.txt".to_owned(),
            needle,
        }),
        1 => mixed_text(300).prop_map(|text| Criterion::ModelJudgement { text }),
    ]
    .boxed()
}

pub(crate) fn events_strategy(max_len: usize) -> BoxedStrategy<Vec<TaskEvent>> {
    prop::collection::vec(known_event(), 0..max_len)
        .prop_map(move |kinds| {
            let mut out = Vec::new();
            out.push(event_with_seq(
                1,
                TaskEventKind::Known(KnownTaskEventKind::TaskCreated {
                    goal: "g".to_owned(),
                    opened_by: anybot_protocol::TaskOpener::You,
                    bot: anybot_foundation::BotId::generate(&SystemClock),
                    parent: None,
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
                    let follow = if idx.is_multiple_of(5) {
                        TaskEventKind::Known(KnownTaskEventKind::ToolFailed {
                            call_id,
                            name,
                            args_summary,
                            error: "err".to_owned(),
                        })
                    } else if idx % 4 != 3 {
                        TaskEventKind::Known(KnownTaskEventKind::ToolCompleted {
                            call_id,
                            summary: "ok".to_owned(),
                            raw_id: None,
                        })
                    } else {
                        TaskEventKind::Known(KnownTaskEventKind::ToolIntent {
                            call_id,
                            name,
                            args_summary,
                            level,
                        })
                    };
                    out.push(event_with_seq(seq.saturating_add(1_000), follow));
                }
            }
            out
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
                    user_turns: renumbered(users),
                    bot_turns: renumbered(bots),
                    steps: unique_step_refs(steps),
                    constraints,
                    open_questions: questions,
                    tool_specs: (0..spec_count).map(tool_spec_at).collect(),
                    system_prompt: "sys".to_owned(),
                    pinned: PinnedSnapshot {
                        version: "v1".to_owned(),
                        items: vec!["pin".to_owned()],
                    },
                    memories: unique_memory_ids(memories),
                    prior_chat: renumbered(prior),
                    model,
                    compact_requested: compact,
                }
            },
        )
        .boxed()
}

pub(crate) use crate::fixtures_events::{event_with_seq, evidence_fixture, known_event};
