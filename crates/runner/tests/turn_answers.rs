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

use std::sync::Arc;
use std::time::Duration;

use anybot_foundation::TaskId;
use anybot_protocol::{CardAnswer, CardButton, EvidenceKind, KnownTaskEventKind};
use anybot_runner::TurnOutcome;
use anybot_trust::{ActionKey, GateConfig, Opinion, RuleEntry};
use runner_test::card_turn::{AskedTurn, send_to_card};
use runner_test::second_bot::second_bot_runner;
use runner_test::trust_world::{
    MAC, cards_of, first_card, first_turn, mac_setup, param, rejected_reason, unticked,
};
use runner_test::turn::{Device, TurnWorld, WorldSetup, arc_world};
use runner_test::turn_helpers::{task_kinds, task_of};
use runner_test::turn_tool_helpers::{call, one_tool_step};
use test_harness::{advance_clock, sleep_real};

const MAKE: &str = "{\"command\":\"make test\"}";

fn ran_in(world: &TurnWorld, task: &TaskId) -> bool {
    task_kinds(world, task).iter().any(|k| {
        matches!(k, KnownTaskEventKind::EvidenceAdded { record } if record.kind == EvidenceKind::Executed)
    })
}

fn ran(world: &TurnWorld, outcome: &TurnOutcome) -> bool {
    ran_in(world, &task_of(outcome))
}

fn shown(world: &TurnWorld, task: &TaskId) -> usize {
    cards_of(&task_kinds(world, task)).len()
}

async fn again(world: &TurnWorld, tool: &str, args: &str, id: &str) -> Result<TurnOutcome, String> {
    let _advanced = advance_clock(&world.clock, Duration::from_millis(100));
    world.script(one_tool_step(vec![call(tool, args, id)]));
    world
        .send(Device::Phone, "again")
        .await
        .map_err(|e| format!("{e:?}"))
}

async fn again_card(
    world: &Arc<TurnWorld>,
    tool: &str,
    args: &str,
    id: &str,
) -> Result<AskedTurn, String> {
    let _advanced = advance_clock(&world.clock, Duration::from_millis(100));
    world.script(one_tool_step(vec![call(tool, args, id)]));
    send_to_card(world, Device::Phone, "again").await
}

async fn mac_world(setup: &WorldSetup) -> Result<Arc<TurnWorld>, String> {
    Ok(arc_world(TurnWorld::start(setup).await?))
}

#[tokio::test]
async fn allow_conversation_lasts_until_new_conversation() -> Result<(), String> {
    let world = mac_world(&mac_setup(MAC)).await?;
    let first = first_card(&world, "shell", MAKE).await?;
    first.answer(&world, CardAnswer::AllowConversation).await?;
    for id in ["c2", "c3"] {
        let next = again(&world, "shell", MAKE, id).await?;
        assert_eq!(shown(&world, &task_of(&next)), 0);
        assert!(ran(&world, &next));
    }
    assert!(world.trust.always_entries().is_empty());
    world.runner().start_new_conversation();
    let fresh = again_card(&world, "shell", MAKE, "c4").await?;
    assert_eq!(shown(&world, &fresh.task), 1);
    fresh.answer(&world, CardAnswer::Deny).await?;
    Ok(())
}

#[tokio::test]
async fn always_allow_covers_other_bot() -> Result<(), String> {
    let world = mac_world(&mac_setup(MAC)).await?;
    let first = first_card(&world, "shell", MAKE).await?;
    first.answer(&world, CardAnswer::AlwaysAllow).await?;
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
    assert_eq!(shown(&world, &task_of(&outcome)), 0);
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
    let world = mac_world(&setup).await?;
    let first = first_card(&world, "shell", MAKE).await?;
    first.answer(&world, CardAnswer::AlwaysAllow).await?;
    assert_eq!(world.trust.ask_rule_count(), 0);
    assert_eq!(world.trust.always_entries().len(), 1);
    let next = again(&world, "shell", MAKE, "c2").await?;
    assert_eq!(shown(&world, &task_of(&next)), 0);
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
    let world = mac_world(&setup).await?;
    let first = first_card(&world, "shell", MAKE).await?;
    let kinds = task_kinds(&world, &first.task);
    let Some(KnownTaskEventKind::CardShown { buttons, .. }) = cards_of(&kinds).first().cloned()
    else {
        return Err("missing card".to_owned());
    };
    assert_eq!(buttons, vec![CardButton::AllowOnce, CardButton::Deny]);
    world
        .answer_shown_card(first.task, CardAnswer::AlwaysAllow)
        .await?;
    sleep_real(Duration::from_millis(200)).await;
    assert!(first.is_running());
    assert!(world.trust.always_entries().is_empty());
    assert!(!ran_in(&world, &first.task));
    let outcome = first.answer(&world, CardAnswer::AllowOnce).await?;
    assert!(ran(&world, &outcome));
    assert!(world.trust.always_entries().is_empty());
    Ok(())
}

#[tokio::test]
async fn switch_card_always_allow_ticks_computer() -> Result<(), String> {
    let world = mac_world(&mac_setup(MAC)).await?;
    world.trust.set_use(MAC, unticked());
    let args = "{\"command\":\"ls\"}";
    let first = first_card(&world, "shell", args).await?;
    first.answer(&world, CardAnswer::AlwaysAllow).await?;
    assert!(world.trust.use_of(MAC).usable_by_bot);
    let next = again(&world, "shell", args, "c2").await?;
    assert_eq!(shown(&world, &task_of(&next)), 0);
    assert!(next.pending.is_empty());
    Ok(())
}

#[tokio::test]
async fn switch_card_direct_mode_turns_on_direct() -> Result<(), String> {
    let world = mac_world(&mac_setup(MAC)).await?;
    world.trust.set_use(MAC, unticked());
    let args = "{\"path\":\"/home/alice/work/a.md\"}";
    let first = first_card(&world, "read_file", args).await?;
    let outcome = first.answer(&world, CardAnswer::DirectMode).await?;
    assert!(world.gate_config().modes.direct_mode);
    let done = task_kinds(&world, &task_of(&outcome))
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
    let world = mac_world(&mac_setup(MAC)).await?;
    let first = first_card(&world, "shell", MAKE).await?;
    first.answer(&world, CardAnswer::AlwaysAllow).await?;
    let covered = again(&world, "shell", MAKE, "c2").await?;
    assert_eq!(shown(&world, &task_of(&covered)), 0);
    let entries = world.trust.always_entries();
    for entry in &entries {
        world.trust.remove_always(&entry.action);
    }
    let asked = again_card(&world, "shell", MAKE, "c3").await?;
    assert_eq!(shown(&world, &asked.task), 1);
    asked.answer(&world, CardAnswer::Deny).await?;
    Ok(())
}
