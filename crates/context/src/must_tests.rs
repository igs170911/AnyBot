use crate::compile::compile;
use crate::compile_tests_support as sup;
use crate::input::{BuildInput, PinnedSnapshot, StepRecord};
use crate::output::{BlockForm, BlockKind, Compiled};

fn input_with(
    base: &BuildInput,
    steps: Vec<StepRecord>,
    user_turns: Vec<crate::input::ChatRecord>,
    usable: u64,
) -> BuildInput {
    BuildInput {
        view_limit: base.view_limit,
        events: base.events.clone(),
        user_turns,
        bot_turns: base.bot_turns.clone(),
        steps,
        constraints: base.constraints.clone(),
        open_questions: base.open_questions.clone(),
        tool_specs: base.tool_specs.clone(),
        system_prompt: base.system_prompt.clone(),
        pinned: PinnedSnapshot {
            version: "v1".to_owned(),
            items: Vec::new(),
        },
        memories: base.memories.clone(),
        prior_chat: base.prior_chat.clone(),
        model: sup::plan(usable),
        compact_requested: false,
        secrets: Vec::new(),
    }
}

#[test]
fn user_turns_survive_many_steps() {
    let base = sup::base_input();
    let users: Vec<crate::input::ChatRecord> = (0..12)
        .map(|i| sup::record(&format!("u{i}"), &format!("user line {i}")))
        .collect();
    let steps: Vec<StepRecord> = (0..80)
        .map(|i| sup::step_len(i, 60, &format!("r{i}")))
        .collect();
    let input = input_with(&base, steps, users.clone(), 32_000);
    match compile(&input).unwrap() {
        Compiled::Sent { request, .. } => {
            for u in &users {
                assert!(
                    request
                        .blocks
                        .iter()
                        .any(|b| b.kind == BlockKind::UserText && b.text == u.text)
                );
            }
        }
        Compiled::Insufficient(_) => panic!("expected sent"),
    }
}

#[test]
fn long_user_turn_head_length_id() {
    let base = sup::base_input();
    let text = "w".repeat(50_000);
    let input = input_with(&base, Vec::new(), vec![sup::record("ubig", &text)], 32_000);
    match compile(&input).unwrap() {
        Compiled::Sent {
            request, manifest, ..
        } => {
            let refs: Vec<&crate::output::Block> = request
                .blocks
                .iter()
                .filter(|b| b.kind == BlockKind::UserRef)
                .collect();
            assert_eq!(refs.len(), 1);
            let r = refs.first().unwrap();
            assert!(r.text.contains(&"w".repeat(1_000)));
            assert!(r.text.contains("len 50000"));
            assert!(r.source_ids.iter().any(|id| id == "ubig"));
            assert!(
                manifest
                    .included
                    .iter()
                    .any(|e| e.kind == BlockKind::UserRef && e.form == BlockForm::Reference)
            );
        }
        Compiled::Insufficient(_) => panic!("expected sent"),
    }
}

#[test]
fn must_docs_and_pending_actions_are_mandatory() {
    let base = sup::base_input();
    let mut input = input_with(&base, Vec::new(), Vec::new(), 32_000);
    input.tool_specs = vec![sup::tool_spec("read_result"), sup::tool_spec("run")];
    input.constraints = vec!["c1".to_owned(), "c2".to_owned()];
    input.open_questions = vec!["what colour?".to_owned()];
    input.events = vec![sup::known(
        2,
        anybot_protocol::KnownTaskEventKind::CardShown {
            action: "what colour?".to_owned(),
        },
    )];
    match compile(&input).unwrap() {
        Compiled::Sent { request, .. } => {
            assert!(
                request
                    .blocks
                    .iter()
                    .any(|b| b.kind == BlockKind::ToolDoc && b.text.contains("read_result"))
            );
            assert!(
                request
                    .blocks
                    .iter()
                    .any(|b| b.kind == BlockKind::OpenQuestion && b.text.contains("what colour?"))
            );
            for c in ["c1", "c2"] {
                assert!(
                    request
                        .blocks
                        .iter()
                        .any(|b| b.kind == BlockKind::Constraint && b.text == c)
                );
            }
        }
        Compiled::Insufficient(_) => panic!("expected sent"),
    }
}

#[test]
fn model_swap_preserves_criteria() {
    let base = sup::base_input();
    let mut input = input_with(&base, Vec::new(), Vec::new(), 32_000);
    input.constraints = vec!["m".to_owned(); 400];
    let criterion = anybot_protocol::Criterion::ModelJudgement {
        text: "j".repeat(24_000),
    };
    let mut events = base.events.clone();
    events.push(sup::known(
        2,
        anybot_protocol::KnownTaskEventKind::DoneWhenChange {
            by: anybot_protocol::Proposer::User,
            change: anybot_protocol::DoneWhenChange::SetItems {
                items: vec![criterion.clone()],
            },
            before: Vec::new(),
            after: vec![criterion],
            reason: "r".to_owned(),
        },
    ));
    input.events = events;
    match compile(&input).unwrap() {
        Compiled::Sent { request, .. } => {
            let needle = "j".repeat(24_000);
            assert!(
                request
                    .blocks
                    .iter()
                    .any(|b| b.kind == BlockKind::Criterion && b.text.contains(&needle)),
                "criteria must survive the swap to a bigger model"
            );
        }
        Compiled::Insufficient(_) => panic!("bigger model must fit"),
    }
}

#[test]
fn huge_criteria_survive_tiny_model() {
    let base = sup::base_input();
    let mut input = input_with(&base, Vec::new(), Vec::new(), 4_000);
    input.constraints = vec!["m".to_owned(); 400];
    let criterion = anybot_protocol::Criterion::ModelJudgement {
        text: "j".repeat(24_000),
    };
    let mut events = base.events.clone();
    events.push(sup::known(
        2,
        anybot_protocol::KnownTaskEventKind::DoneWhenChange {
            by: anybot_protocol::Proposer::User,
            change: anybot_protocol::DoneWhenChange::SetItems {
                items: vec![criterion.clone()],
            },
            before: Vec::new(),
            after: vec![criterion],
            reason: "r".to_owned(),
        },
    ));
    input.events = events;
    match compile(&input).unwrap() {
        Compiled::Insufficient(x) => {
            assert_eq!(x.message.code, crate::compile::CODE_MODEL_BUDGET);
            assert!(x.deficit_tokens > 0);
        }
        Compiled::Sent { .. } => panic!("mandatory content must not fit"),
    }
}
