use anybot_context as _;
use anybot_domain as _;
use anybot_foundation as _;
use anybot_messages as _;
use anybot_models as _;
use anybot_runner as _;
use anybot_store_client as _;
use anybot_tools as _;
use proptest as _;
use serde_json as _;
use sha2 as _;
use test_harness as _;
use thiserror as _;

use anybot_protocol::{CardAnswer, CardButton, EvidenceKind, KnownTaskEventKind};
use anybot_runner::TurnOutcome;
use anybot_trust::{ActionKey, GateConfig, Opinion, RuleEntry};
use runner_test::second_bot::second_bot_runner;
use runner_test::trust_world::{
    MAC, cards_of, first_turn, mac_setup, param, rejected_reason, unticked,
};
use runner_test::turn::{Device, TurnWorld, WorldSetup};
use runner_test::turn_helpers::{answer_card, shown_card, task_kinds, task_of};
use runner_test::turn_tool_helpers::{call, one_tool_step};
use test_harness::advance_clock;

const MAKE: &str = "{\"line\":\"make test\"}";

fn ran(world: &TurnWorld, outcome: &TurnOutcome) -> bool {
    task_kinds(world, &task_of(outcome)).iter().any(|k| {
        matches!(k, KnownTaskEventKind::EvidenceAdded { record } if record.kind == EvidenceKind::Executed)
    })
}

fn shown(world: &TurnWorld, outcome: &TurnOutcome) -> usize {
    cards_of(&task_kinds(world, &task_of(outcome))).len()
}

async fn again(world: &TurnWorld, tool: &str, args: &str, id: &str) -> Result<TurnOutcome, String> {
    let _advanced = advance_clock(&world.clock, std::time::Duration::from_millis(100));
    world.script(one_tool_step(vec![call(tool, args, id)]));
    world
        .send(Device::Phone, "again")
        .await
        .map_err(|e| format!("{e:?}"))
}

async fn answer_first(
    world: &TurnWorld,
    outcome: &TurnOutcome,
    answer: CardAnswer,
) -> Result<(), String> {
    let kinds = task_kinds(world, &task_of(outcome));
    let digest = shown_card(outcome, &kinds).ok_or("missing digest")?;
    answer_card(world, outcome, &digest, answer).await
}

#[tokio::test]
async fn allow_conversation_lasts_until_new_conversation() -> Result<(), String> {
    let world = TurnWorld::start(&mac_setup(MAC)).await?;
    let first = first_turn(&world, "shell", MAKE).await?;
    answer_first(&world, &first, CardAnswer::AllowConversation).await?;
    for id in ["c2", "c3"] {
        let next = again(&world, "shell", MAKE, id).await?;
        assert_eq!(shown(&world, &next), 0);
        assert!(ran(&world, &next));
    }
    assert!(world.trust.always_entries().is_empty());
    world.runner().start_new_conversation();
    let fresh = again(&world, "shell", MAKE, "c4").await?;
    assert_eq!(shown(&world, &fresh), 1);
    Ok(())
}

#[tokio::test]
async fn always_allow_covers_other_bot() -> Result<(), String> {
    let world = TurnWorld::start(&mac_setup(MAC)).await?;
    let first = first_turn(&world, "shell", MAKE).await?;
    answer_first(&world, &first, CardAnswer::AlwaysAllow).await?;
    let entries = world.trust.always_entries();
    assert_eq!(entries.len(), 1);
    let bot1 = entries.first().ok_or("missing entry")?;
    assert!(bot1.action.contains("make"));
    let other = second_bot_runner(&world, "Bot2")?;
    world.script(one_tool_step(vec![call("shell", MAKE, "c9")]));
    let outcome = other
        .on_user_message(anybot_runner::UserMessage {
            device: world.device(Device::Phone),
            text: "build".to_owned(),
        })
        .await
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(shown(&world, &outcome), 0);
    assert!(ran(&world, &outcome));
    assert_eq!(world.trust.always_entries().len(), 1);
    Ok(())
}

#[tokio::test]
async fn ask_rule_always_allow_cancels_rule() -> Result<(), String> {
    let mut setup: WorldSetup = mac_setup(MAC);
    let mut gate = GateConfig::plain();
    gate.user
        .ask_style
        .push(ActionKey::command("shell", "make test"));
    setup.gate = gate;
    let world = TurnWorld::start(&setup).await?;
    let first = first_turn(&world, "shell", MAKE).await?;
    answer_first(&world, &first, CardAnswer::AlwaysAllow).await?;
    assert_eq!(world.trust.ask_rule_count(), 0);
    assert_eq!(world.trust.always_entries().len(), 1);
    let next = again(&world, "shell", MAKE, "c2").await?;
    assert_eq!(shown(&world, &next), 0);
    assert!(ran(&world, &next));
    Ok(())
}

#[tokio::test]
async fn admin_card_refuses_always_answer() -> Result<(), String> {
    let mut setup = mac_setup(MAC);
    let mut gate = GateConfig::plain();
    gate.admin.hook.push(RuleEntry {
        key: ActionKey::command("shell", "make test"),
        opinion: Opinion::Ask,
    });
    setup.gate = gate;
    let world = TurnWorld::start(&setup).await?;
    let first = first_turn(&world, "shell", MAKE).await?;
    let kinds = task_kinds(&world, &task_of(&first));
    let Some(KnownTaskEventKind::CardShown { buttons, .. }) = cards_of(&kinds).first().cloned()
    else {
        return Err("missing card".to_owned());
    };
    assert_eq!(buttons, vec![CardButton::AllowOnce, CardButton::Deny]);
    let refused = answer_first(&world, &first, CardAnswer::AlwaysAllow).await;
    assert!(refused.is_err_and(|e| e.contains("does not offer")));
    assert!(world.trust.always_entries().is_empty());
    let answered_before = task_kinds(&world, &task_of(&first))
        .iter()
        .filter(|k| matches!(k, KnownTaskEventKind::CardAnswered { .. }))
        .count();
    assert_eq!(answered_before, 0);
    answer_first(&world, &first, CardAnswer::AllowOnce).await?;
    assert!(ran(&world, &first));
    Ok(())
}

#[tokio::test]
async fn switch_card_always_allow_ticks_computer() -> Result<(), String> {
    let world = TurnWorld::start(&mac_setup(MAC)).await?;
    world.trust.set_use(MAC, unticked());
    let args = "{\"line\":\"ls\"}";
    let first = first_turn(&world, "shell", args).await?;
    answer_first(&world, &first, CardAnswer::AlwaysAllow).await?;
    assert!(world.trust.use_of(MAC).usable_by_bot);
    let next = again(&world, "shell", args, "c2").await?;
    assert_eq!(shown(&world, &next), 0);
    assert!(next.pending.is_empty());
    Ok(())
}

#[tokio::test]
async fn switch_card_direct_mode_turns_on_direct() -> Result<(), String> {
    let world = TurnWorld::start(&mac_setup(MAC)).await?;
    world.trust.set_use(MAC, unticked());
    let args = "{\"path\":\"/home/alice/work/a.md\"}";
    let first = first_turn(&world, "read_file", args).await?;
    answer_first(&world, &first, CardAnswer::DirectMode).await?;
    assert!(world.gate_config().modes.direct_mode);
    let done = task_kinds(&world, &task_of(&first))
        .iter()
        .any(|k| matches!(k, KnownTaskEventKind::ToolCompleted { summary, .. } if summary.contains("a.md")));
    assert!(done);
    Ok(())
}

#[tokio::test]
async fn unreadable_always_entry_denies_step() -> Result<(), String> {
    let world = TurnWorld::start(&mac_setup(MAC)).await?;
    world.trust.set_unreadable(Some("shell:git push"));
    let outcome = first_turn(&world, "shell", MAKE).await?;
    let kinds = task_kinds(&world, &task_of(&outcome));
    let reason = rejected_reason(&kinds).ok_or("missing Rejected")?;
    assert_eq!(reason.code, "gate-rule-unreadable");
    assert_eq!(param(&reason, "rule"), Some("shell:git push"));
    assert!(cards_of(&kinds).is_empty());
    Ok(())
}

#[tokio::test]
async fn removed_always_entry_asks_next_step() -> Result<(), String> {
    let world = TurnWorld::start(&mac_setup(MAC)).await?;
    let first = first_turn(&world, "shell", MAKE).await?;
    answer_first(&world, &first, CardAnswer::AlwaysAllow).await?;
    let covered = again(&world, "shell", MAKE, "c2").await?;
    assert_eq!(shown(&world, &covered), 0);
    let entries = world.trust.always_entries();
    for entry in &entries {
        world.trust.remove_always(&entry.action);
    }
    let asked = again(&world, "shell", MAKE, "c3").await?;
    assert_eq!(shown(&world, &asked), 1);
    Ok(())
}
