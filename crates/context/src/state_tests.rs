use proptest::prelude::*;

use anybot_foundation::{CallId, SystemClock};

use crate::compile::compile;
use crate::compile_tests_support as sup;
use crate::fixtures::events_strategy;
use crate::numbers::K114_7_STATE_MAX_TOKENS;
use crate::output::{BlockKind, Compiled};
use crate::state::build_state;
use crate::state_render::{fit_summary, render};
#[test]
fn state_starts_empty() {
    let b = build_state(&[]);
    let s = render(&b, false);
    assert!(s.items.is_empty());
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    #[test]
    fn t_context_state_1(events in events_strategy(24)) {
        let a = fit_summary(&build_state(&events));
        let b = fit_summary(&build_state(&events));
        prop_assert_eq!(&a, &b);
        let seqs: Vec<u64> = events.iter().map(|e| e.seq).collect();
        for item in &a.items {
            prop_assert!(seqs.contains(&item.source_seq));
        }
        let compress = render(&build_state(&events), true);
        prop_assert!(compress.items.len() <= a.items.len());
    }
}

fn state_2_events() -> Vec<anybot_protocol::TaskEvent> {
    let mut events: Vec<anybot_protocol::TaskEvent> = vec![sup::known(
        1,
        anybot_protocol::KnownTaskEventKind::TaskCreated {
            goal: "g".to_owned(),
        },
    )];
    for i in 0..20u64 {
        let seq = i.saturating_add(2);
        events.push(sup::known(
            seq,
            anybot_protocol::KnownTaskEventKind::ToolIntent {
                call_id: CallId::generate(&SystemClock),
                name: "t".to_owned(),
                args_summary: "a".to_owned(),
                level: anybot_protocol::ToolEffect::ReadOnly,
            },
        ));
        events.push(sup::known(
            seq.saturating_add(1),
            anybot_protocol::KnownTaskEventKind::ToolCompleted {
                call_id: CallId::generate(&SystemClock),
            },
        ));
        if i < 3 {
            events.push(sup::known(
                seq,
                anybot_protocol::KnownTaskEventKind::EvidenceAdded {
                    summary: format!("file f{i} sha {i}"),
                },
            ));
        }
    }
    events.push(sup::known(
        22,
        anybot_protocol::KnownTaskEventKind::EvidenceAdded {
            summary: "command exit code 1".to_owned(),
        },
    ));
    events
}

#[test]
fn t_context_state_2() {
    let mut base = sup::base_input();
    base.events = state_2_events();
    match compile(&base) {
        Compiled::Sent { request, .. } => {
            let summaries: Vec<&crate::output::Block> = request
                .blocks
                .iter()
                .filter(|b| b.kind == BlockKind::StateSummary)
                .collect();
            assert_eq!(summaries.len(), 1, "one state summary");
            let text = summaries.first().expect("summary").text.clone();
            for i in 0..3 {
                assert!(text.contains(&format!("file f{i} sha {i}")));
            }
            assert!(text.contains("exit code 1"));
            assert!(
                request
                    .blocks
                    .iter()
                    .any(|b| b.kind == BlockKind::CurrentStep),
                "current step present"
            );
        }
        Compiled::Insufficient(_) => panic!("must fit"),
    }
}

#[test]
fn t_context_state_3() {
    let mut base = sup::base_input();
    let mut events = base.events.clone();
    events.push(sup::known(
        2,
        anybot_protocol::KnownTaskEventKind::DoneWhenChange {
            by: anybot_protocol::Proposer::User,
            change: anybot_protocol::DoneWhenChange::SetItems {
                items: vec![anybot_protocol::Criterion::CommandExitZero {
                    command: "cargo test".to_owned(),
                }],
            },
            before: Vec::new(),
            after: vec![anybot_protocol::Criterion::CommandExitZero {
                command: "cargo test".to_owned(),
            }],
            reason: "r".to_owned(),
        },
    ));
    base.events = events;
    base.bot_turns = vec![sup::record("b1", "all tests passed, task done")];
    match compile(&base) {
        Compiled::Sent { request, .. } => {
            let summary_items: Vec<&str> = request
                .blocks
                .iter()
                .filter(|b| b.kind == BlockKind::StateSummary)
                .map(|b| b.text.as_str())
                .collect();
            let summary = summary_items.join("");
            assert!(
                !summary.contains("all tests passed"),
                "claims are not state"
            );
            assert!(summary.contains("pending"), "criterion not verified");
        }
        Compiled::Insufficient(_) => panic!("must fit"),
    }
}

#[test]
fn t_context_state_4() {
    let mut events: Vec<anybot_protocol::TaskEvent> = Vec::new();
    events.push(sup::known(
        1,
        anybot_protocol::KnownTaskEventKind::TaskCreated {
            goal: "g".to_owned(),
        },
    ));
    let mut open_ids: Vec<String> = Vec::new();
    for i in 0..300u64 {
        let call = sup::step_len(usize::try_from(i).unwrap_or(0), 10, "x").call_id;
        let seq = i.saturating_add(2);
        events.push(sup::known(
            seq,
            anybot_protocol::KnownTaskEventKind::ToolIntent {
                call_id: call,
                name: "t".to_owned(),
                args_summary: "a".to_owned(),
                level: anybot_protocol::ToolEffect::ReadOnly,
            },
        ));
        if i < 280 {
            events.push(sup::known(
                seq.saturating_add(1),
                anybot_protocol::KnownTaskEventKind::ToolCompleted { call_id: call },
            ));
        } else {
            open_ids.push(call.as_str().clone());
        }
    }
    for q in 0..20u64 {
        events.push(sup::known(
            q.saturating_add(400),
            anybot_protocol::KnownTaskEventKind::CardShown {
                action: format!("question {q}"),
            },
        ));
    }
    let summary = render(&build_state(&events), true);
    let text: String = summary
        .items
        .iter()
        .map(|i| i.text.clone())
        .collect::<Vec<String>>()
        .join("\n");
    assert!(text.contains("completed: 280"), "done count only");
    for id in &open_ids {
        assert!(text.contains(id.as_str()), "all open steps listed");
    }
    for q in 0..20 {
        assert!(text.contains(&format!("question {q}")));
    }
    let tokens = crate::estimate::text_tokens(&text);
    assert!(
        tokens <= K114_7_STATE_MAX_TOKENS,
        "summary within token budget"
    );
}
