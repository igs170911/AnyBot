use crate::budget_tests::assemble_input;
use crate::compile::compile;
use crate::compile_tests_support as sup;

use crate::output::{BlockKind, CollapseReason, Compiled, NewEvent};

#[test]
fn budget_drop_takes_oldest_results_then_stops() {
    let steps: Vec<_> = (0..8)
        .map(|i| sup::step_len(i, 4_000, &format!("r{i}")))
        .collect();
    let prior: Vec<_> = (0..4)
        .map(|i| sup::record(&format!("p{i}"), &"p".repeat(800)))
        .collect();
    let input = assemble_input(steps, 8_000, Vec::new(), prior);
    let Compiled::Sent {
        request, events, ..
    } = compile(&input).unwrap()
    else {
        panic!("expected sent");
    };
    let dropped = budget_ids(&events);
    assert_eq!(
        dropped,
        vec![
            "r0".to_owned(),
            "r1".to_owned(),
            "r2".to_owned(),
            "r3".to_owned(),
            "r4".to_owned()
        ]
    );
    assert!(request.blocks.iter().any(|b| {
        b.kind == BlockKind::ToolResultFull && b.source_ids.iter().any(|id| id == "result-r6")
    }));
    assert!(request.blocks.iter().any(|b| {
        b.kind == BlockKind::PriorChatFull && b.source_ids.iter().any(|id| id == "p0")
    }));
    assert_eq!(budget_events(&events), 1);
    short_prior_also_drops_oldest();
}

fn short_prior_also_drops_oldest() {
    let prior: Vec<_> = (0..4)
        .map(|i| sup::record(&format!("p{i}"), &"q".repeat(8_000)))
        .collect();
    let input = assemble_input(Vec::new(), 8_000, Vec::new(), prior);
    let Compiled::Sent {
        request, events, ..
    } = compile(&input).unwrap()
    else {
        panic!("expected sent");
    };
    assert_eq!(
        budget_ids(&events),
        vec!["p0".to_owned(), "p1".to_owned(), "p2".to_owned()]
    );
    assert!(request.blocks.iter().any(|b| {
        b.kind == BlockKind::PriorChatFull && b.source_ids.iter().any(|id| id == "p3")
    }));
    assert_eq!(budget_events(&events), 1);
}

#[test]
fn deficit_counts_prefix_blocks_once() {
    let mut input = sup::base_input();
    input.model = sup::plan(2_200);
    input.system_prompt = "s".repeat(400);
    input.tool_specs = vec![sup::tool_spec("read_file")];
    input.constraints = vec!["c".repeat(8_000)];
    let Compiled::Insufficient(info) = compile(&input).unwrap() else {
        panic!("expected insufficient");
    };
    assert_eq!(info.deficit_tokens, 1_249);
}

fn card_shown(action: &str) -> anybot_protocol::KnownTaskEventKind {
    anybot_protocol::KnownTaskEventKind::CardShown {
        action: action.to_owned(),
        card: "card-1".to_owned(),
        kind: anybot_protocol::CardKind::Action,
        resume: None,
        buttons: vec![
            anybot_protocol::CardButton::AllowOnce,
            anybot_protocol::CardButton::Deny,
        ],
        reason: anybot_protocol::ReasonPayload {
            code: "card-run-command".to_owned(),
            params: Vec::new(),
        },
        always_label: None,
        notes: Vec::new(),
        classes: Vec::new(),
    }
}

#[test]
fn answered_card_is_not_an_open_question() {
    let mut input = sup::base_input();
    input.events = vec![
        sup::known(1, card_shown("which colour?")),
        sup::known(
            2,
            anybot_protocol::KnownTaskEventKind::CardAnswered {
                answer: anybot_protocol::CardAnswer::AllowOnce,
                origin: anybot_protocol::AnswerOrigin::Hand,
            },
        ),
        sup::known(3, card_shown("which file?")),
    ];
    let Compiled::Sent { request, .. } = compile(&input).unwrap() else {
        panic!("expected sent");
    };
    let questions: Vec<&str> = request
        .blocks
        .iter()
        .filter(|b| b.kind == BlockKind::OpenQuestion)
        .map(|b| b.text.as_str())
        .collect();
    assert_eq!(questions, vec!["open question: which file?"]);
}

#[test]
fn swap_plus_compact_redecides_from_scratch() {
    let steps: Vec<_> = (0..12)
        .map(|i| sup::step_len(i, 700, &format!("r{i}")))
        .collect();
    let mut input = assemble_input(steps, 80_000, Vec::new(), Vec::new());
    input.compact_requested = true;
    input.events = vec![
        sup::event(
            1,
            anybot_domain::collapsed_kind(&anybot_domain::ContextCollapsed {
                ids: vec!["r0".to_owned()],
                reason: anybot_domain::ContextCollapseReason::Accumulated,
            }),
        ),
        sup::known(
            2,
            anybot_protocol::KnownTaskEventKind::ModelSwapped {
                from: "old".to_owned(),
                to: "new".to_owned(),
                reason: "handoff".to_owned(),
            },
        ),
    ];
    let Compiled::Sent { events, .. } = compile(&input).unwrap() else {
        panic!("expected sent");
    };
    let NewEvent::Collapsed { ids, reason } = events.first().expect("collapse") else {
        panic!("expected collapse");
    };
    assert_eq!(*reason, CollapseReason::ReplaceAll);
    assert_eq!(ids, &vec!["r0".to_owned(), "r1".to_owned()]);
}

fn budget_ids(events: &[NewEvent]) -> Vec<String> {
    let mut ids = Vec::new();
    for event in events {
        if let NewEvent::Collapsed {
            ids: dropped,
            reason: CollapseReason::Budget,
        } = event
        {
            ids.extend(dropped.clone());
        }
    }
    ids
}

fn budget_events(events: &[NewEvent]) -> usize {
    events
        .iter()
        .filter(|event| {
            matches!(
                event,
                NewEvent::Collapsed {
                    reason: CollapseReason::Budget,
                    ..
                }
            )
        })
        .count()
}
