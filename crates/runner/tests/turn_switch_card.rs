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

use anybot_messages::{Lang, Params};
use anybot_protocol::{CardAnswer, CardButton, CardKind, KnownTaskEventKind, TurnEndReason};
use runner_test::trust_world::{
    MAC, ask_every_time, card_reason, cards_of, first_turn, mac_setup, modes, param, render,
    unticked,
};
use runner_test::turn::TurnWorld;
use runner_test::turn_helpers::{
    answer_card, answer_pending, fmt_text, shown_card, task_kinds, task_of,
};

#[tokio::test]
async fn switch_decline_text_is_recorded_for_the_model() -> Result<(), String> {
    let world = TurnWorld::start(&mac_setup(MAC)).await?;
    world.trust.set_use(MAC, unticked());
    let outcome = first_turn(
        &world,
        "read_file",
        "{\"path\":\"/home/alice/Documents/a.txt\"}",
    )
    .await?;
    assert_eq!(outcome.reason, TurnEndReason::PendingApproval);
    let task = task_of(&outcome);
    let kinds = task_kinds(&world, &task);
    let cards = cards_of(&kinds);
    let Some(KnownTaskEventKind::CardShown {
        kind,
        buttons,
        reason,
        ..
    }) = cards.first()
    else {
        return Err("missing CardShown".to_owned());
    };
    assert_eq!(*kind, CardKind::Switch);
    assert_eq!(
        buttons,
        &vec![
            CardButton::AllowOnce,
            CardButton::AllowConversation,
            CardButton::AlwaysAllow,
            CardButton::Deny,
            CardButton::DirectMode,
        ]
    );
    assert_eq!(reason.code, "card-switch");
    assert_eq!(param(reason, "bot"), Some("Bot1"));
    assert_eq!(param(reason, "computer"), Some(MAC));
    let digest = shown_card(&outcome, &kinds).ok_or("missing digest")?;
    answer_card(&world, &outcome, &digest, CardAnswer::Deny).await?;
    let expected = fmt_text(
        Lang::ZhTw,
        "gate-switch-declined-bot",
        &Params::new()
            .with_text("bot", "Bot1")
            .with_text("computer", MAC),
    )?;
    assert!(expected.starts_with("\u{4f7f}\u{7528}\u{8005}\u{6c92}\u{6709}\u{5141}\u{8a31} Bot1 "));
    let recorded = task_kinds(&world, &task).iter().any(
        |k| matches!(k, KnownTaskEventKind::ToolCompleted { summary, .. } if summary == &expected),
    );
    assert!(recorded, "missing decline text in the record");
    Ok(())
}

#[tokio::test]
async fn direct_mode_skips_switch_card() -> Result<(), String> {
    let mut setup = mac_setup(MAC);
    setup.gate = modes(false, true);
    let world = TurnWorld::start(&setup).await?;
    world.trust.set_use(MAC, unticked());
    let outcome = first_turn(&world, "read_file", "{\"path\":\"/home/alice/work/a.md\"}").await?;
    let kinds = task_kinds(&world, &task_of(&outcome));
    assert!(cards_of(&kinds).is_empty());
    assert!(
        kinds
            .iter()
            .any(|k| matches!(k, KnownTaskEventKind::ToolCompleted { .. }))
    );
    Ok(())
}

#[tokio::test]
async fn t_sandbox_levels_2() -> Result<(), String> {
    let calls = [
        (
            "edit_file",
            "{\"path\":\"a.md\",\"old_text\":\"old\",\"new_text\":\"new\"}",
            "card-edit-file",
            "\u{4fee}\u{6539}\u{6a94}\u{6848}\u{ff1a}",
        ),
        (
            "shell",
            "{\"line\":\"make test\"}",
            "card-run-command",
            "\u{57f7}\u{884c}\u{6307}\u{4ee4}\u{ff1a}make test",
        ),
        (
            "read_file",
            "{\"path\":\"~/Downloads/b.txt\"}",
            "card-read-outside",
            "\u{8b80}\u{53d6}\u{5de5}\u{4f5c}\u{8cc7}\u{6599}\u{593e}\u{4ee5}\u{5916}\u{7684}\u{5730}\u{65b9}\u{ff1a}",
        ),
    ];
    for (tool, args, code, shown_prefix) in calls {
        let world = TurnWorld::start(&mac_setup(MAC)).await?;
        world.trust.set_use(MAC, ask_every_time());
        let outcome = first_turn(&world, tool, args).await?;
        let cards = cards_of(&task_kinds(&world, &task_of(&outcome)));
        assert_eq!(cards.len(), 1);
        let (reason, label, kind) = card_reason(&cards)?;
        assert_eq!(reason.code, code);
        assert_ne!(kind, CardKind::Switch);
        let shown = render(&reason)?;
        assert!(
            shown.starts_with(shown_prefix),
            "unexpected card text: {shown}"
        );
        if tool == "shell" {
            let label = label.ok_or("missing always label")?;
            assert_eq!(label.code, "always-run-command");
            assert_eq!(
                render(&label)?,
                "\u{5728}\u{300c}Work Mac\u{300d}\u{8dd1} make"
            );
        }
    }
    Ok(())
}

#[tokio::test]
async fn switch_then_action_card() -> Result<(), String> {
    let world = TurnWorld::start(&mac_setup(MAC)).await?;
    world.trust.set_use(MAC, unticked());
    let outcome = first_turn(
        &world,
        "edit_file",
        "{\"path\":\"a.md\",\"old_text\":\"old\",\"new_text\":\"new\"}",
    )
    .await?;
    let task = task_of(&outcome);
    let kinds = task_kinds(&world, &task);
    let digest = shown_card(&outcome, &kinds).ok_or("missing digest")?;
    answer_card(&world, &outcome, &digest, CardAnswer::AllowOnce).await?;
    let next = world.runner().take_parked_cards();
    assert_eq!(next.len(), 1);
    let cards = cards_of(&task_kinds(&world, &task));
    let first = card_reason(&cards)?;
    assert_eq!(first.2, CardKind::Switch);
    let Some(KnownTaskEventKind::CardShown { kind, .. }) = cards.get(1) else {
        return Err("missing second card".to_owned());
    };
    assert_ne!(*kind, CardKind::Switch);
    let second = next.first().ok_or("missing parked card")?;
    answer_pending(&world, task, outcome.turn, second, CardAnswer::AllowOnce).await?;
    assert_eq!(
        world.tree_content("/home/alice/work/a.md"),
        Some("new".to_owned())
    );
    Ok(())
}
