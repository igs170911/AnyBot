use proptest::prelude::*;

use anybot_foundation::{CallId, SystemClock};
use anybot_protocol::{BeforeDigest, EvidencePayload, KnownTaskEventKind, TaskEventKind};

use crate::compile::compile;
use crate::compile_tests_support as sup;
use crate::fixtures::events_strategy;
use crate::numbers::K114_7_STATE_MAX_TOKENS;
use crate::output::{BlockKind, Compiled};
use crate::state::build_state;
use crate::state_render::{StateSummary, fit_summary, render};

#[test]
fn state_starts_empty() {
    let b = build_state(&[]);
    assert!(render(&b, false).items.is_empty());
}

fn check_evidence_item(
    summary: &StateSummary,
    seq: u64,
    record: &anybot_protocol::EvidenceRecord,
) -> Result<(), TestCaseError> {
    let has = |pred: &dyn Fn(&str) -> bool| {
        summary
            .items
            .iter()
            .any(|i| i.source_seq == seq && pred(&i.text))
    };
    match &record.payload {
        EvidencePayload::Changed {
            path,
            before,
            after,
            ..
        } => {
            let after_hex = after.to_string();
            let before_str = match before {
                BeforeDigest::Absent => "absent".to_owned(),
                BeforeDigest::Present(v) => v.to_string(),
            };
            let ok =
                has(&|t| t.contains(path) && t.contains(&after_hex) && t.contains(&before_str));
            prop_assert!(ok);
        }
        EvidencePayload::Seen { path, digest, .. } => {
            let hex = digest.to_string();
            prop_assert!(has(&|t| t.contains(path) && t.contains(&hex)));
        }
        EvidencePayload::Executed { command, .. } => {
            prop_assert!(has(&|t| t.contains(command)));
        }
        EvidencePayload::Approved { .. }
        | EvidencePayload::Rejected { .. }
        | EvidencePayload::NotExecuted { .. } => {}
    }
    Ok(())
}

fn check_state_invariants(
    events: &[anybot_protocol::TaskEvent],
    a: &StateSummary,
) -> Result<(), TestCaseError> {
    let seqs: Vec<u64> = events.iter().map(|e| e.seq).collect();
    for item in &a.items {
        prop_assert!(seqs.contains(&item.source_seq));
    }
    for e in events {
        if let TaskEventKind::Known(KnownTaskEventKind::EvidenceAdded { record }) = &e.kind {
            check_evidence_item(a, e.seq, record)?;
        }
        if let TaskEventKind::Known(KnownTaskEventKind::TaskCreated { goal, .. }) = &e.kind {
            prop_assert!(
                a.items
                    .iter()
                    .any(|i| i.source_seq == e.seq && i.text.contains(goal))
            );
        }
    }
    Ok(())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    #[test]
    fn t_context_state_1(events in events_strategy(180)) {
        let st = build_state(&events);
        let a = fit_summary(&st);
        let b = fit_summary(&st);
        prop_assert_eq!(&a, &b);
        check_state_invariants(&events, &a)?;
        let full = render(&st, false);
        let compress = render(&st, true);
        prop_assert!(compress.items.len() <= full.items.len());
        if full.tokens() > K114_7_STATE_MAX_TOKENS {
            prop_assert_eq!(&a, &compress);
        }
    }
}

fn executed_fixture(exit: i32) -> anybot_protocol::EvidenceRecord {
    let mut record = crate::fixtures::evidence_fixture(9);
    record.tool = "shell".to_owned();
    record.payload = EvidencePayload::Executed {
        command: "run".to_owned(),
        computer: "c".to_owned(),
        exit_code: Some(exit),
        output_head: String::new(),
        output_tail: String::new(),
        output_omitted: 0,
        full_digest: anybot_foundation::ContentDigest::sha256(b"o"),
        elapsed_ms: 1,
    };
    record
}

fn state_2_events() -> Vec<anybot_protocol::TaskEvent> {
    let mut events = vec![sup::known(
        1,
        KnownTaskEventKind::TaskCreated {
            goal: "g".to_owned(),
            opened_by: anybot_protocol::TaskOpener::You,
            bot: anybot_foundation::BotId::generate(&SystemClock),
            parent: None,
        },
    )];
    for i in 0..20u64 {
        let seq = i.saturating_add(2);
        events.push(sup::known(
            seq,
            KnownTaskEventKind::ToolIntent {
                call_id: CallId::generate(&SystemClock),
                name: "t".to_owned(),
                args_summary: "a".to_owned(),
                level: anybot_protocol::ToolEffect::ReadOnly,
            },
        ));
        events.push(sup::known(
            seq.saturating_add(1),
            KnownTaskEventKind::ToolCompleted {
                call_id: CallId::generate(&SystemClock),
                summary: "s".to_owned(),
                raw_id: None,
            },
        ));
        if i < 3 {
            let ev = crate::fixtures::evidence_fixture(u32::try_from(i).unwrap_or(0));
            events.push(sup::known(
                seq,
                KnownTaskEventKind::EvidenceAdded {
                    record: Box::new(ev),
                },
            ));
        }
    }
    events.push(sup::known(
        22,
        KnownTaskEventKind::EvidenceAdded {
            record: Box::new(executed_fixture(1)),
        },
    ));
    events
}

#[test]
fn compiled_request_has_one_state_summary() {
    let mut base = sup::base_input();
    base.events = state_2_events();
    let Compiled::Sent { request, .. } = compile(&base).unwrap() else {
        panic!("must fit");
    };
    let summaries: Vec<&crate::output::Block> = request
        .blocks
        .iter()
        .filter(|b| b.kind == BlockKind::StateSummary)
        .collect();
    assert_eq!(summaries.len(), 1);
    let text = &summaries.first().expect("summary").text;
    assert!((0..3).all(|i| text.contains(&format!("seen f{i}"))));
    assert!(text.contains("exit 1"));
    assert!(
        request
            .blocks
            .iter()
            .any(|b| b.kind == BlockKind::CurrentStep)
    );
}

#[test]
fn model_reply_does_not_enter_state_summary() {
    let mut base = sup::base_input();
    let crit = anybot_protocol::Criterion::CommandExitZero {
        command: "cargo test".to_owned(),
    };
    base.events.push(sup::done_when_event(2, vec![crit]));
    base.bot_turns = vec![sup::record("b1", "all tests passed, task done")];
    let Compiled::Sent { request, .. } = compile(&base).unwrap() else {
        panic!("must fit");
    };
    let summary: String = request
        .blocks
        .iter()
        .filter(|b| b.kind == BlockKind::StateSummary)
        .map(|b| b.text.as_str())
        .collect();
    assert!(!summary.contains("all tests passed"));
    assert!(summary.contains("pending"));
}

fn seed_plan_events(
    events: &mut Vec<anybot_protocol::TaskEvent>,
    done_ids: &mut Vec<String>,
    open_ids: &mut Vec<String>,
) {
    events.push(sup::known(
        1,
        KnownTaskEventKind::TaskCreated {
            goal: "g".to_owned(),
            opened_by: anybot_protocol::TaskOpener::You,
            bot: anybot_foundation::BotId::generate(&SystemClock),
            parent: None,
        },
    ));
    for i in 0..300u64 {
        let call = sup::step_len(usize::try_from(i).unwrap_or(0), 10, "x").call_id;
        let seq = i.saturating_add(2);
        events.push(sup::known(
            seq,
            KnownTaskEventKind::ToolIntent {
                call_id: call,
                name: "t".to_owned(),
                args_summary: "a".to_owned(),
                level: anybot_protocol::ToolEffect::ReadOnly,
            },
        ));
        if i < 280 {
            events.push(sup::known(
                seq.saturating_add(1),
                KnownTaskEventKind::ToolCompleted {
                    call_id: call,
                    summary: "s".to_owned(),
                    raw_id: None,
                },
            ));
            done_ids.push(call.as_str().clone());
        } else {
            open_ids.push(call.as_str().clone());
        }
    }
    for q in 0..20u64 {
        events.push(sup::known(
            q.saturating_add(400),
            KnownTaskEventKind::CardShown {
                action: format!("question {q}"),
                card: format!("card-{q}"),
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
            },
        ));
    }
}

#[test]
fn t_context_state_4() {
    let mut events = Vec::new();
    let mut done_ids = Vec::new();
    let mut open_ids = Vec::new();
    seed_plan_events(&mut events, &mut done_ids, &mut open_ids);
    assert_eq!(done_ids.len(), 280);
    assert_eq!(open_ids.len(), 20);
    let st = build_state(&events);
    assert!(render(&st, false).tokens() > K114_7_STATE_MAX_TOKENS);
    let summary = fit_summary(&st);
    let text = summary.text();
    assert!(text.contains("completed: 280 failed: 0"));
    assert_eq!(text.matches(" open [seq").count(), 20);
    assert!(done_ids.iter().all(|id| !text.contains(id.as_str())));
    assert!(open_ids.iter().all(|id| text.contains(id.as_str())));
    assert!((0..20).all(|q| text.contains(&format!("question {q}"))));
    assert!(summary.tokens() <= K114_7_STATE_MAX_TOKENS);
    assert!(crate::estimate::text_tokens(&text) <= K114_7_STATE_MAX_TOKENS);
}
