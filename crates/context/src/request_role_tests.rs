use proptest::prelude::*;

use crate::compile::compile;
use crate::compile_tests_support as sup;
use crate::fixtures::build_input;
use crate::output::{BlockKind, CODE_TURN_PROGRAM_NOTE, Compiled, CompiledRequest};
use anybot_models::{Role, ThinkingMode};

fn prefix_joined(request: &CompiledRequest) -> String {
    request
        .blocks
        .iter()
        .filter(|b| {
            matches!(
                b.kind,
                BlockKind::System
                    | BlockKind::ToolDoc
                    | BlockKind::Pinned
                    | BlockKind::Constraint
                    | BlockKind::Criterion
            )
        })
        .map(|b| b.text.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}

fn same_prefix_input() -> crate::input::BuildInput {
    let mut input = sup::base_input();
    input.system_prompt = "SAME-SYS".to_owned();
    input.tool_specs = vec![sup::tool_spec("read_file")];
    input.user_turns = vec![sup::record("u1", "hello")];
    input
}

fn sent(input: &crate::input::BuildInput) -> CompiledRequest {
    match compile(input).unwrap() {
        Compiled::Sent { request, .. } => request,
        Compiled::Insufficient(_) => panic!("expected sent"),
    }
}

fn turn(request: &CompiledRequest) -> anybot_models::TurnRequest {
    request
        .to_turn_request(Vec::new(), ThinkingMode::Auto, None)
        .expect("turn")
}

fn unpaired_intent(request: &CompiledRequest) -> bool {
    let mut open: Vec<&str> = Vec::new();
    for block in &request.blocks {
        if block.kind == BlockKind::ToolIntent {
            let Some(id) = block.call_id.as_deref() else {
                return true;
            };
            if block.tool_name.is_none() || block.arguments.is_none() {
                return true;
            }
            open.push(id);
        }
        if matches!(
            block.kind,
            BlockKind::ToolResultFull
                | BlockKind::ToolResultMasked
                | BlockKind::ToolResultDuplicate
        ) {
            let Some(id) = block.call_id.as_deref() else {
                return true;
            };
            let Some(pos) = open.iter().position(|known| *known == id) else {
                return true;
            };
            open.remove(pos);
        }
    }
    !open.is_empty()
}

fn ends_on_user_or_tool(request: &CompiledRequest) -> Result<bool, ()> {
    let Ok(built) = request.to_turn_request(Vec::new(), ThinkingMode::Auto, None) else {
        return Err(());
    };
    Ok(match built.messages.last() {
        None => true,
        Some(last) => matches!(last.role, Role::User | Role::Tool),
    })
}

#[test]
fn prefix_lives_in_system_and_tail_is_the_last_user_message() {
    let mut input = sup::base_input();
    input.system_prompt = "SYS-MARK".to_owned();
    input.tool_specs = vec![sup::tool_spec("read_file")];
    input.pinned.items = vec!["PIN-MARK".to_owned()];
    input.constraints = vec!["LIMIT-MARK".to_owned()];
    input.user_turns = vec![sup::record("u1", "USER-MARK")];
    input.memories = vec![sup::memory("m1", 9, false)];
    input.events = vec![sup::known(
        1,
        anybot_protocol::KnownTaskEventKind::CardShown {
            action: "ASK-MARK".to_owned(),
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
        },
    )];
    let request = sent(&input);
    let built = turn(&request);
    assert!(built.system.contains("SYS-MARK"));
    assert!(built.system.contains("read_file"));
    assert!(built.system.contains("PIN-MARK"));
    assert!(built.system.contains("LIMIT-MARK"));
    assert!(built.messages.iter().all(|m| {
        !m.text.contains("SYS-MARK")
            && !m.text.contains("PIN-MARK")
            && !m.text.contains("LIMIT-MARK")
            && !m.text.contains("read_file description")
    }));
    let last = built.messages.last().expect("tail");
    assert_eq!(last.role, Role::User);
    assert!(last.text.starts_with(CODE_TURN_PROGRAM_NOTE));
    assert!(last.text.contains("ASK-MARK"));
    assert!(last.text.contains("memory m1"));
    assert!(built.messages.iter().any(|m| m.text == "USER-MARK"));
}

#[test]
fn system_bytes_hold_when_only_the_tail_changes() {
    let first = same_prefix_input();
    let mut second = same_prefix_input();
    second.events = vec![sup::known(
        1,
        anybot_protocol::KnownTaskEventKind::CardShown {
            action: "later question".to_owned(),
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
        },
    )];
    let sent_a = sent(&first);
    let sent_b = sent(&second);
    let a = turn(&sent_a);
    let b = turn(&sent_b);
    let prefix_a = prefix_joined(&sent_a);
    let prefix_b = prefix_joined(&sent_b);
    assert_eq!(prefix_a, prefix_b);
    assert_eq!(a.system, prefix_a);
    assert_eq!(b.system, prefix_b);
    assert_ne!(
        a.messages.last().map(|m| m.text.as_str()),
        b.messages.last().map(|m| m.text.as_str())
    );
}

#[test]
fn missing_arguments_and_orphan_results_are_errors() {
    let mut missing = sup::base_input();
    let step = sup::step_len(0, 40, "r0");
    let mut request = sent_with_step(&mut missing, step);
    for block in &mut request.blocks {
        if block.kind == BlockKind::ToolIntent {
            block.arguments = None;
        }
    }
    let err = request
        .to_turn_request(Vec::new(), ThinkingMode::Auto, None)
        .expect_err("missing arguments");
    assert!(format!("{err:?}").contains("context-tool-pair"));

    let mut orphan = sup::base_input();
    let mut bare = sup::step_len(0, 40, "r1");
    bare.call_id =
        anybot_foundation::CallId::parse("00000000-0000-0000-0000-000000000009").expect("call id");
    let request = sent_with_step(&mut orphan, bare);
    let CompiledRequest { mut blocks, .. } = request;
    blocks.retain(|b| b.kind != BlockKind::ToolIntent);
    let request = CompiledRequest {
        system: "sys".to_owned(),
        prefix_len: 1,
        blocks,
    };
    let err = request
        .to_turn_request(Vec::new(), ThinkingMode::Auto, None)
        .expect_err("orphan result");
    assert!(format!("{err:?}").contains("context-tool-pair"));
}

fn sent_with_step(
    input: &mut crate::input::BuildInput,
    step: crate::input::StepRecord,
) -> CompiledRequest {
    input.steps = vec![step];
    sent(input)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn compiled_turn_never_ends_on_the_bot(
        input in build_input(8_000, 80_000, 6),
        drop_tail in prop::bool::ANY,
        drop_result in prop::bool::ANY,
    ) {
        let mut input = input;
        input.steps = crate::fixtures::unique_step_refs(input.steps);
        let mut request = match compile(&input).unwrap() {
            Compiled::Sent { request, .. } => request,
            Compiled::Insufficient(_) => return Ok(()),
        };
        if drop_tail {
            request.blocks.retain(|b| {
                !matches!(
                    b.kind,
                    BlockKind::StateSummary
                        | BlockKind::CurrentStep
                        | BlockKind::OpenQuestion
                        | BlockKind::PendingAction
                        | BlockKind::Memory
                )
            });
        }
        if drop_result {
            let call = request
                .blocks
                .iter()
                .rev()
                .find(|b| b.kind == BlockKind::ToolResultFull)
                .and_then(|b| b.call_id.clone());
            if let Some(id) = call {
                let mut dropped = false;
                request.blocks.retain(|b| {
                    let full = b.kind == BlockKind::ToolResultFull && b.call_id.as_deref() == Some(id.as_str());
                    if full && !dropped {
                        dropped = true;
                        return false;
                    }
                    !(b.kind == BlockKind::ToolIntent && b.call_id.as_deref() == Some(id.as_str()))
                });
            }
        }
        match ends_on_user_or_tool(&request) {
            Ok(ended) => prop_assert!(ended),
            Err(()) => prop_assert!(unpaired_intent(&request)),
        }
    }
}
