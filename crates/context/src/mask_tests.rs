use crate::compile::compile;
use crate::compile_tests_support as sup;
use crate::input::StepRecord;
use crate::mask::render_view;
use crate::numbers::K114_2_DEFAULT_RESULT_CHARS;
use crate::output::{BlockKind, Compiled, NewEvent};

fn line(count: usize, marker: &str) -> String {
    let parts: Vec<String> = (0..count).map(|n| format!("line {n} {marker}\n")).collect();
    parts.join("")
}

fn big_step(len: usize, unconfirmed: bool, outcome: crate::input::StepOutcomeKind) -> StepRecord {
    StepRecord {
        content: "y".repeat(len),
        unconfirmed,
        outcome,
        ..sup::step_len(0, 0, "z")
    }
}

#[test]
fn long_output_head_tail_error() {
    let content = line(5_000, "filler");
    let error_line = "line 3000 ERROR boom\n".to_owned();
    let positions: Vec<usize> = content
        .char_indices()
        .nth(2_999)
        .map(|(idx, _)| vec![idx])
        .unwrap_or_default();
    let step = StepRecord {
        seq: 1,
        call_id: sup::step_len(0, 0, "r").call_id,
        tool: "cmd".to_owned(),
        args_summary: "run".to_owned(),
        level: anybot_protocol::ToolEffect::ReadOnly,
        unconfirmed: false,
        outcome: crate::input::StepOutcomeKind::Failed,
        source_ref: "ref-5000".to_owned(),
        content: content + &error_line,
        exit_code: Some(1),
        error_at: positions,
    };
    let view = render_view(&step, K114_2_DEFAULT_RESULT_CHARS);
    assert!(view.starts_with("line 0 filler"));
    assert!(view.contains("line 3000"));
    assert!(view.contains("ERROR boom"));
    assert!(view.contains("elided"));
    assert!(view.contains("ref-5000"));
    let summary = crate::mask::result_summary(&step);
    assert!(summary.contains("status=fail"));
    assert!(summary.contains("exit=1"));
}

#[test]
fn duplicate_result_shown_once() {
    let mut base = sup::base_input();
    let shared = "same content here".repeat(30);
    let mut first = sup::step_len(0, shared.chars().count(), "ra");
    first.content.clone_from(&shared);
    let mut second = sup::step_len(1, shared.chars().count(), "rb");
    second.content.clone_from(&shared);
    base.steps = vec![first, second];
    match compile(&base).unwrap() {
        Compiled::Sent {
            request, manifest, ..
        } => {
            let fulls: Vec<&crate::output::Block> = request
                .blocks
                .iter()
                .filter(|b| b.kind == BlockKind::ToolResultFull)
                .collect();
            assert_eq!(fulls.len(), 1);
            assert!(
                request
                    .blocks
                    .iter()
                    .any(|b| b.kind == BlockKind::ToolResultDuplicate)
            );
            assert!(
                manifest
                    .excluded
                    .iter()
                    .any(|e| e.reason == crate::output::ExcludedReason::Duplicate)
            );
        }
        Compiled::Insufficient(_) => panic!("expected sent"),
    }
}

#[test]
fn long_user_turn_becomes_reference() {
    let mut base = sup::base_input();
    base.model = sup::plan(4_000);
    base.user_turns = vec![sup::record("u1", &"z".repeat(60_000))];
    base.constraints = Vec::new();
    match compile(&base).unwrap() {
        Compiled::Sent {
            request, manifest, ..
        } => {
            let refs: Vec<&crate::output::Block> = request
                .blocks
                .iter()
                .filter(|b| b.kind == BlockKind::UserRef)
                .collect();
            assert_eq!(refs.len(), 1);
            assert!(
                refs.first()
                    .is_some_and(|r| r.text.contains(&"z".repeat(1_000)))
            );
            assert!(refs.first().is_some_and(|r| r.text.contains("len 60000")));
            assert!(refs.first().is_some_and(|r| r.text.contains("u1")));
            assert!(
                manifest
                    .included
                    .iter()
                    .any(|e| e.kind == BlockKind::UserRef
                        && e.form == crate::output::BlockForm::Reference)
            );
        }
        Compiled::Insufficient(_) => panic!("expected sent"),
    }
}

#[test]
fn rebuild_identical_after_collapse_event() {
    let mut base = sup::base_input();
    base.events = vec![sup::known(
        1,
        anybot_protocol::KnownTaskEventKind::TaskCreated {
            goal: "g".to_owned(),
        },
    )];
    base.steps = vec![sup::step_len(0, 500, "ra")];
    let first = compile(&base).unwrap();
    let second = compile(&base).unwrap();
    let bytes = |c: &Compiled| match c {
        Compiled::Sent { request, .. } => serde_json::to_vec(request).unwrap(),
        Compiled::Insufficient(x) => serde_json::to_vec(x).unwrap(),
    };
    assert_eq!(bytes(&first), bytes(&second));
}

#[test]
fn compact_plan_collapses_every_old_result() {
    let mut base = sup::base_input();
    base.compact_requested = true;
    base.steps = (0..14)
        .map(|i| sup::step_len(i, 2_000, &format!("r{i}")))
        .collect();
    match compile(&base).unwrap() {
        Compiled::Sent {
            request, events, ..
        } => {
            let old_masked: usize = request
                .blocks
                .iter()
                .filter(|b| b.kind == BlockKind::ToolResultMasked)
                .count();
            assert!(old_masked >= 4);
            assert!(events.iter().any(|e| matches!(
                e,
                NewEvent::Collapsed {
                    reason: crate::output::CollapseReason::Compact,
                    ..
                }
            )));
        }
        Compiled::Insufficient(_) => panic!("expected sent"),
    }
}

#[test]
fn compact_skips_unconfirmed_short_and_unknown() {
    let mut base = sup::base_input();
    base.compact_requested = true;
    base.steps = vec![
        big_step(50, true, crate::input::StepOutcomeKind::Completed),
        big_step(50, false, crate::input::StepOutcomeKind::Unknown),
        big_step(120, false, crate::input::StepOutcomeKind::Completed),
        big_step(3_000, false, crate::input::StepOutcomeKind::Completed),
    ];
    match compile(&base).unwrap() {
        Compiled::Sent {
            request, events, ..
        } => {
            assert!(
                events.is_empty()
                    || !events.iter().any(|e| matches!(
                        e,
                        NewEvent::Collapsed { ids, .. } if ids.contains(&"ref0".to_owned())
                    ))
            );
            let masked: Vec<String> = request
                .blocks
                .iter()
                .filter(|b| b.kind == BlockKind::ToolResultMasked)
                .flat_map(|b| b.source_ids.clone())
                .collect();
            assert!(
                !masked
                    .iter()
                    .any(|id| id == "ref0" || id == "ref1" || id == "ref2")
            );
        }
        Compiled::Insufficient(_) => panic!("expected sent"),
    }
}
