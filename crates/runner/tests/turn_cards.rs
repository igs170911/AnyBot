use anybot_context as _;
use anybot_domain as _;
use anybot_foundation as _;
use anybot_models as _;
use anybot_runner as _;
use anybot_tools as _;
use anybot_trust as _;
use proptest as _;
use serde_json as _;
use sha2 as _;
use test_harness as _;
use thiserror as _;

use std::time::Duration;

use anybot_foundation::{BotId, SystemClock, TimestampMs};
use anybot_messages::{Lang, Params};
use anybot_protocol::{
    CardAnswer, CardButton, CardKind, ComputerOwner, ComputerPlatform, KnownTaskEventKind,
    TurnEndReason,
};
use anybot_trust::{AlwaysEntry, AlwaysList, GateConfig};
use runner_test::fake_scripts::CommandScript;
use runner_test::turn::{Device, TurnWorld, WorldSetup};
use runner_test::turn_helpers::{answer_card, fmt_text, shown_card, task_kinds, task_of};
use runner_test::turn_tool_helpers::{all_specs, call, edit_call, one_tool_step};
use test_harness::advance_clock;

fn user_mac_setup() -> WorldSetup {
    WorldSetup {
        owner: ComputerOwner::User,
        platform: ComputerPlatform::Mac,
        computer_name: "Work Mac".to_owned(),
        bot_name: "Bot1".to_owned(),
        workdir: "/home/alice/work".to_owned(),
        home_dir: "/home/alice".to_owned(),
        tools: all_specs(),
        files: vec![
            (
                "/home/alice/work/.git/hooks/pre-commit".to_owned(),
                "#!/bin/sh\n".to_owned(),
            ),
            (
                "/home/alice/.zshrc".to_owned(),
                "export PATH=.\n".to_owned(),
            ),
        ],
        ..WorldSetup::default()
    }
}

fn no_sandbox_setup() -> WorldSetup {
    WorldSetup {
        owner: ComputerOwner::User,
        platform: ComputerPlatform::Windows,
        computer_name: "Win Box".to_owned(),
        bot_name: "Bot1".to_owned(),
        sandbox: false,
        tools: all_specs(),
        scripts: vec![
            ("ls".to_owned(), CommandScript::ok("a.txt")),
            ("type .env".to_owned(), CommandScript::ok("SECRET=1")),
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

fn check_note_zh_tw(code: &str, expected: &str) -> Result<(), String> {
    let rendered = fmt_text(Lang::ZhTw, code, &Params::new())?;
    if rendered != expected {
        return Err(format!("expected {expected:?}, got {rendered:?}"));
    }
    Ok(())
}

#[tokio::test]
async fn t_sandbox_levels_3() -> Result<(), String> {
    let mut setup = user_mac_setup();
    setup.gate = GateConfig {
        user: anybot_trust::UserRules {
            always: AlwaysList {
                entries: vec![AlwaysEntry {
                    action: "command make on Work Mac".to_owned(),
                    added_at: TimestampMs::from_millis(1),
                    from_bot: BotId::generate(&SystemClock),
                }],
            },
            ..anybot_trust::UserRules::default()
        },
        ..GateConfig::plain()
    };
    let world = TurnWorld::start(&setup).await?;
    world.script(one_tool_step(vec![call(
        "shell",
        "{\"line\":\"make test && npm run x && pip install y\"}",
        "c1",
    )]));
    let outcome = world
        .send(Device::Phone, "build all")
        .await
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(outcome.reason, TurnEndReason::PendingApproval);
    let kinds = task_kinds(&world, &task_of(&outcome));
    let cards = find_cards(&kinds);
    let Some(KnownTaskEventKind::CardShown {
        kind,
        buttons,
        reason,
        always_label,
        notes,
        ..
    }) = cards.first()
    else {
        return Err("missing CardShown".to_owned());
    };
    assert_eq!(*kind, CardKind::EveryTime);
    assert_eq!(buttons, &vec![CardButton::AllowOnce, CardButton::Deny]);
    assert_eq!(reason.code, "card-run-command");
    assert!(always_label.is_none());
    assert!(notes.iter().any(|n| n.code == "card-note-every-time"));
    check_note_zh_tw(
        "card-note-every-time",
        "\u{9019}\u{985e}\u{64cd}\u{4f5c}\u{6bcf}\u{6b21}\u{90fd}\u{6703}\u{554f}\u{4f60}",
    )?;
    Ok(())
}

#[tokio::test]
async fn t_sandbox_n4_1() -> Result<(), String> {
    let world = TurnWorld::start(&no_sandbox_setup()).await?;
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
    let kinds = task_kinds(&world, &task_of(&outcome));
    let cards = find_cards(&kinds);
    let Some(KnownTaskEventKind::CardShown {
        kind,
        buttons,
        notes,
        always_label,
        ..
    }) = cards.first()
    else {
        return Err("missing CardShown".to_owned());
    };
    assert_eq!(*kind, CardKind::EveryTime);
    assert_eq!(buttons, &vec![CardButton::AllowOnce, CardButton::Deny]);
    assert!(always_label.is_none());
    assert!(
        notes
            .iter()
            .any(|n| n.code == "card-note-no-sandbox-private")
    );
    assert!(notes.iter().any(|n| n.code == "card-note-every-time"));
    Ok(())
}

#[tokio::test]
async fn t_sandbox_n2_2() -> Result<(), String> {
    let world = TurnWorld::start(&user_mac_setup()).await?;
    world.script(one_tool_step(vec![edit_call(
        "{\"path\":\".git/hooks/pre-commit\",\"old_text\":\"#!/bin/sh\\n\",\"new_text\":\"#!/bin/sh\\nid\\n\"}",
        "c1",
    )]));
    let first = world
        .send(Device::Phone, "hook")
        .await
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(first.reason, TurnEndReason::PendingApproval);
    let task = task_of(&first);
    let digest = shown_card(&first, &task_kinds(&world, &task)).ok_or("missing first card")?;
    answer_card(&world, &first, &digest, CardAnswer::Deny).await?;
    let _advanced = advance_clock(&world.clock, Duration::from_millis(100));
    world.script(one_tool_step(vec![edit_call(
        "{\"path\":\"~/.zshrc\",\"old_text\":\"export PATH=.\\n\",\"new_text\":\"export PWN=1\\n\"}",
        "c2",
    )]));
    let second = world
        .send(Device::Phone, "zshrc")
        .await
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(second.reason, TurnEndReason::PendingApproval);
    let mut cards = find_cards(&task_kinds(&world, &task));
    cards.extend(find_cards(&task_kinds(&world, &task_of(&second))));
    assert_eq!(cards.len(), 2);
    for card in &cards {
        let KnownTaskEventKind::CardShown {
            kind,
            buttons,
            reason,
            ..
        } = card
        else {
            return Err("unexpected event".to_owned());
        };
        assert_eq!(*kind, CardKind::EveryTime);
        assert_eq!(buttons, &vec![CardButton::AllowOnce, CardButton::Deny]);
        assert_eq!(reason.code, "card-edit-outside");
    }
    Ok(())
}

#[tokio::test]
async fn t_sandbox_private_5() -> Result<(), String> {
    let world = TurnWorld::start(&no_sandbox_setup()).await?;
    world.script(one_tool_step(vec![call(
        "shell",
        "{\"line\":\"type .env\"}",
        "c1",
    )]));
    let outcome = world
        .send(Device::Phone, "env")
        .await
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(outcome.reason, TurnEndReason::PendingApproval);
    let cards = find_cards(&task_kinds(&world, &task_of(&outcome)));
    let Some(KnownTaskEventKind::CardShown {
        kind,
        buttons,
        notes,
        ..
    }) = cards.first()
    else {
        return Err("missing CardShown".to_owned());
    };
    assert_eq!(*kind, CardKind::EveryTime);
    assert_eq!(buttons, &vec![CardButton::AllowOnce, CardButton::Deny]);
    assert!(
        notes
            .iter()
            .any(|n| n.code == "card-note-no-sandbox-private")
    );
    check_note_zh_tw(
        "card-note-no-sandbox-private",
        "\u{9019}\u{53f0}\u{96fb}\u{8166}\u{6c92}\u{6709}\u{6307}\u{4ee4}\u{6c99}\u{76d2}\u{ff1a}\u{9019}\u{689d}\u{6307}\u{4ee4}\u{8b80}\u{5f97}\u{5230}\u{4f60}\u{7684}\u{79c1}\u{5bc6}\u{6a94}",
    )?;
    Ok(())
}
