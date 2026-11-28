use crate::compile::compile;
use crate::compile_tests_support as sup;
use crate::input::StepRecord;
use crate::mask::render_view;
use crate::numbers::K114_2_DEFAULT_RESULT_CHARS;
use crate::output::{BlockKind, Compiled, NewEvent};

fn big_step(len: usize, unconfirmed: bool, outcome: crate::input::StepOutcomeKind) -> StepRecord {
    StepRecord {
        content: "y".repeat(len),
        unconfirmed,
        outcome,
        ..sup::step_len(0, 0, "z")
    }
}

fn count_elided_and_kept(view: &str, source_ref: &str) -> (usize, usize, usize) {
    let mut parts = view.split("[elided ");
    let first = parts.next().unwrap_or("");
    let mut kept_chars = first.chars().count();
    let mut elided_sum = 0usize;
    let mut marker_count = 0usize;
    let suffix = format!(" chars, full text ref={source_ref}] ");
    for part in parts {
        let (count_str, rest) = part.split_once(&suffix).expect("elision marker format");
        let count: usize = count_str.parse().expect("elided count");
        elided_sum = elided_sum.saturating_add(count);
        kept_chars = kept_chars.saturating_add(rest.chars().count());
        marker_count = marker_count.saturating_add(1);
    }
    (marker_count, elided_sum, kept_chars)
}

fn build_5000_line_doc() -> String {
    (1..=5_000)
        .map(|n| {
            if n == 3_000 {
                "L3000 ERROR abcdefg\n".to_owned()
            } else {
                format!("L{n:04} filler abcdef\n")
            }
        })
        .collect()
}

fn assert_mask_3_summary(step: &StepRecord) {
    let summary = crate::mask::result_summary(step);
    assert!(summary.contains("tool=cmd"));
    assert!(summary.contains("args=run"));
    assert!(summary.contains("status=fail"));
    assert!(summary.contains("exit=1"));
    assert!(summary.contains("size=100000"));
    assert!(summary.contains("ref=ref-5000"));
}

#[test]
fn t_context_mask_3() {
    let content = build_5000_line_doc();
    assert_eq!(content.chars().count(), 100_000);
    let error_at = crate::error_line::find_error_line(&content, Some(1));
    assert_eq!(error_at, Some(2_999usize.saturating_mul(20)));
    let step = StepRecord {
        seq: 1,
        call_id: sup::step_len(0, 0, "r").call_id,
        tool: "cmd".to_owned(),
        args_summary: "run".to_owned(),
        arguments: serde_json::json!({ "cmd": "run" }),
        level: anybot_protocol::ToolEffect::ReadOnly,
        unconfirmed: false,
        outcome: crate::input::StepOutcomeKind::Failed,
        source_ref: "ref-5000".to_owned(),
        content,
        exit_code: Some(1),
        error_at,
    };
    let view = render_view(&step, K114_2_DEFAULT_RESULT_CHARS);
    assert!(view.starts_with("L0001 filler abcdef\n"));
    assert!(view.contains("L2999 filler abcdef\n"));
    assert!(view.contains("L3000 ERROR abcdefg\n"));
    assert!(view.contains("L3001 filler abcdef\n"));
    assert!(view.ends_with("L5000 filler abcdef\n"));
    assert!(view.contains("ref-5000"));

    let (marker_count, elided_sum, kept_chars) = count_elided_and_kept(&view, "ref-5000");
    assert_eq!(marker_count, 2);
    assert_eq!(elided_sum, 100_000usize.saturating_sub(kept_chars));
    assert_mask_3_summary(&step);
}

#[test]
fn duplicate_result_points_at_first_step() {
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
fn two_collapse_events_replay_the_same_request() {
    let mut base = sup::base_input();
    base.events = vec![sup::known(
        1,
        anybot_protocol::KnownTaskEventKind::TaskCreated {
            goal: "g".to_owned(),
            opened_by: anybot_protocol::TaskOpener::You,
            bot: anybot_foundation::BotId::generate(&anybot_foundation::SystemClock),
            parent: None,
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
