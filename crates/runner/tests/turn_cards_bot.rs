use anybot_context as _;
use anybot_domain as _;
use anybot_foundation as _;
use anybot_models as _;
use anybot_runner as _;
use anybot_store_client as _;
use anybot_tools as _;
use anybot_trust as _;
use proptest as _;
use serde_json as _;
use sha2 as _;
use test_harness as _;
use thiserror as _;

use std::time::Duration;

use anybot_messages::{Lang, Params};
use anybot_protocol::{
    CardButton, CardKind, ComputerOwner, ComputerPlatform, EvidenceKind, GateOutcome, GateStep,
    KnownTaskEventKind, ReasonPayload, TaskDetail, TaskStatus, TurnEndReason,
};
use runner_test::fake_scripts::CommandScript;
use runner_test::turn::{Device, TurnWorld, WorldSetup};
use runner_test::turn_helpers::{fmt_text, state_of, task_kinds, task_of};
use runner_test::turn_tool_helpers::{call, one_tool_step};
use test_harness::advance_clock;

fn bot_computer_setup() -> WorldSetup {
    WorldSetup {
        owner: Some(ComputerOwner::Bot),
        platform: ComputerPlatform::BotMachine,
        computer_name: "bot-computer".to_owned(),
        bot_name: "Bot1".to_owned(),
        workdir: "/workspace".to_owned(),
        files: vec![("/workspace/a.txt".to_owned(), "temp".to_owned())],
        scripts: vec![
            ("rm a.txt".to_owned(), CommandScript::ok("")),
            ("git push".to_owned(), CommandScript::ok("pushed")),
        ],
        ..WorldSetup::default()
    }
}

fn find_cards(kinds: &[KnownTaskEventKind]) -> Vec<KnownTaskEventKind> {
    kinds
        .iter()
        .filter(|k| matches!(k, KnownTaskEventKind::CardShown { .. }))
        .cloned()
        .collect()
}

fn assert_git_push_card(card: &KnownTaskEventKind) -> Result<(), String> {
    let KnownTaskEventKind::CardShown {
        kind,
        buttons,
        always_label,
        classes,
        notes,
        ..
    } = card
    else {
        return Err("missing CardShown for git push".to_owned());
    };
    assert_eq!(*kind, CardKind::Action);
    assert_eq!(
        buttons,
        &vec![
            CardButton::AllowOnce,
            CardButton::AllowConversation,
            CardButton::AlwaysAllow,
            CardButton::Deny,
        ]
    );
    assert_eq!(classes, &vec!["publish".to_owned()]);
    assert!(notes.is_empty());
    let expected_always = ReasonPayload {
        code: "always-run-command-bot-computer".to_owned(),
        params: vec![("words".to_owned(), "git push".to_owned())],
    };
    assert_eq!(always_label.as_ref(), Some(&expected_always));
    let rendered = fmt_text(
        Lang::ZhTw,
        "always-run-command-bot-computer",
        &Params::new().with_text("words", "git push"),
    )?;
    assert_eq!(
        rendered,
        "\u{5728} Bot \u{7684}\u{96fb}\u{8166}\u{8dd1} git push"
    );
    Ok(())
}

#[tokio::test]
async fn t_safety_n5_12() -> Result<(), String> {
    let world = TurnWorld::start(&bot_computer_setup()).await?;
    world.script(one_tool_step(vec![
        call("shell", "{\"line\":\"rm a.txt\"}", "c1"),
        call("shell", "{\"line\":\"git push\"}", "c2"),
    ]));
    let outcome = world
        .send(Device::Phone, "clean and push")
        .await
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(outcome.reason, TurnEndReason::PendingApproval);
    let kinds = task_kinds(&world, &task_of(&outcome));
    let executed = kinds
        .iter()
        .find_map(|k| {
            if let KnownTaskEventKind::EvidenceAdded { record } = k
                && record.kind == EvidenceKind::Executed
            {
                Some(record.as_ref())
            } else {
                None
            }
        })
        .ok_or("missing Executed evidence for rm")?;
    assert_eq!(executed.gate.rule, "bot-computer-own-folder");
    assert_eq!(executed.gate.gate, GateStep::DefaultRule);
    assert_eq!(executed.gate.outcome, GateOutcome::Allow);
    let cards = find_cards(&kinds);
    assert_eq!(cards.len(), 1);
    let card = cards.first().ok_or("missing CardShown")?;
    assert_git_push_card(card)
}

#[tokio::test]
async fn no_sandbox_card_waits_after_an_hour() -> Result<(), String> {
    let mut setup = WorldSetup {
        owner: Some(ComputerOwner::User),
        platform: ComputerPlatform::Windows,
        computer_name: "Win Box".to_owned(),
        bot_name: "Bot1".to_owned(),
        sandbox: false,
        scripts: vec![("ls".to_owned(), CommandScript::ok("a.txt"))],
        ..WorldSetup::default()
    };
    setup.gate.modes.yolo = true;
    setup.gate.modes.direct_mode = true;
    let world = TurnWorld::start(&setup).await?;
    world.script(one_tool_step(vec![call(
        "shell",
        "{\"line\":\"ls\"}",
        "c1",
    )]));
    let outcome = world
        .send(Device::Phone, "list")
        .await
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(outcome.reason, TurnEndReason::PendingApproval);
    let _advanced = advance_clock(&world.clock, Duration::from_secs(3_600));
    let task = task_of(&outcome);
    let state = state_of(&world, &task)?;
    assert_eq!(state.0, TaskStatus::AwaitingUser);
    assert!(matches!(state.1, Some(TaskDetail::ApprovalPending { .. })));
    let cards = find_cards(&task_kinds(&world, &task));
    let Some(KnownTaskEventKind::CardShown { buttons, notes, .. }) = cards.first() else {
        return Err("missing CardShown".to_owned());
    };
    assert_eq!(buttons, &vec![CardButton::AllowOnce, CardButton::Deny]);
    assert!(
        notes
            .iter()
            .any(|n| n.code == "card-note-no-sandbox-private")
    );
    Ok(())
}
