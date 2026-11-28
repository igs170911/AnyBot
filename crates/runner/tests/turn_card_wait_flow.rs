use anybot_context as _;
use anybot_domain as _;
use anybot_foundation as _;
use anybot_messages as _;
use anybot_models as _;
use anybot_store_client as _;
use anybot_tools as _;
use proptest as _;
use serde_json as _;
use sha2 as _;
use thiserror as _;

use std::sync::Arc;
use std::time::Duration;

use anybot_protocol::{
    CardAnswer, ComputerPlatform, EvidenceKind, GateOutcome, KnownTaskEventKind, NotRunReason,
    RuleLayer, ThinkingChoice, TurnEndReason,
};
use anybot_runner::{StoredBody, TurnError, TurnOutcome};
use anybot_trust::{ActionKey, AlwaysEntry, GateConfig};
use runner_test::fake_scripts::CommandScript;
use runner_test::turn::{Device, TurnWorld, WorldSetup};
use runner_test::turn_helpers::{request_body, task_kinds, wait_for_shown_card};
use runner_test::turn_tool_helpers::{call, edit_call};
use test_harness::{model_replies_text, model_tool_calls, timeout_real};

const MAKE: &str = "make";

async fn join_turn(
    bg: tokio::task::JoinHandle<Result<TurnOutcome, TurnError>>,
) -> Result<TurnOutcome, String> {
    timeout_real(bg, Duration::from_secs(5))
        .await
        .map_err(|e| format!("turn timed out: {e}"))?
        .map_err(|e| format!("join failed: {e}"))?
        .map_err(|e| format!("turn error: {e:?}"))
}

fn refused_middle_setup() -> WorldSetup {
    let mut gate = GateConfig::plain();
    gate.user
        .allow_style
        .push(ActionKey::file("read_file", "read", "a.txt"));
    gate.user
        .ask_style
        .push(ActionKey::file("edit_file", "edit", "b.txt"));
    WorldSetup {
        platform: ComputerPlatform::Linux,
        files: vec![
            ("a.txt".to_owned(), "hello".to_owned()),
            ("b.txt".to_owned(), "world".to_owned()),
        ],
        scripts: vec![(MAKE.to_owned(), CommandScript::ok("built"))],
        gate,
        ..WorldSetup::default()
    }
}

#[tokio::test]
async fn refused_middle_card_skips_remaining_actions_in_same_turn() -> Result<(), String> {
    let world = runner_test::turn::arc_world(TurnWorld::start(&refused_middle_setup()).await?);
    world.script(vec![
        model_tool_calls(
            vec![
                call("read_file", "{\"path\":\"a.txt\"}", "c1"),
                call("shell", "{\"command\":\"make\"}", "c2"),
                edit_call(
                    "{\"path\":\"b.txt\",\"old_text\":\"world\",\"new_text\":\"next\"}",
                    "c3",
                ),
            ],
            "tool_calls",
            false,
        ),
        model_replies_text("stopped after refusal", false),
    ]);
    let shared = runner_test::turn::share(&world);
    let bg = tokio::spawn(async move { shared.send(Device::Phone, "run three").await });

    let (task, _cards) = wait_for_shown_card(&world, 1).await?;
    world.answer_shown_card(task, CardAnswer::Deny).await?;
    let outcome = join_turn(bg).await?;
    assert_eq!(outcome.reason, TurnEndReason::Answered);
    assert_eq!(world.tree_content("b.txt").as_deref(), Some("world"));

    let kinds = task_kinds(&world, &task);
    let shown_count = kinds
        .iter()
        .filter(|k| matches!(k, KnownTaskEventKind::CardShown { .. }))
        .count();
    assert_eq!(shown_count, 1);
    assert!(kinds.iter().any(|k| matches!(
        k,
        KnownTaskEventKind::EvidenceAdded { record } if record.kind == EvidenceKind::Seen
    )));
    assert!(kinds.iter().any(|k| matches!(
        k,
        KnownTaskEventKind::GateResult { gate, .. } if gate.outcome == GateOutcome::Deny
    )));
    assert!(kinds.iter().any(|k| matches!(
        k,
        KnownTaskEventKind::ToolNotRun {
            reason: NotRunReason::PreviousRefused,
            ..
        }
    )));

    let body = request_body(world.model.as_ref(), 1)?;
    assert_eq!(count_tool_messages(&body), 3);
    let text = serde_json::to_string(&body).map_err(|e| format!("{e}"))?;
    assert!(text.contains("previous-refused"));
    Ok(())
}

#[tokio::test]
async fn always_allow_added_mid_step_allows_next_action_without_card() -> Result<(), String> {
    let mut gate = GateConfig::plain();
    gate.user
        .allow_style
        .push(ActionKey::file("read_file", "read", "a.txt"));
    let setup = WorldSetup {
        platform: ComputerPlatform::Linux,
        files: vec![("a.txt".to_owned(), "data".to_owned())],
        scripts: vec![(MAKE.to_owned(), CommandScript::ok("built"))],
        gate,
        ..WorldSetup::default()
    };
    let world = runner_test::turn::arc_world(TurnWorld::start(&setup).await?);
    install_mid_step_always_hook(&world);
    world.script(vec![
        model_tool_calls(
            vec![
                call("read_file", "{\"path\":\"a.txt\"}", "c1"),
                call("shell", "{\"command\":\"make\"}", "c2"),
            ],
            "tool_calls",
            false,
        ),
        model_replies_text("all done", false),
    ]);

    let outcome = timeout_real(
        world.send(Device::Phone, "read then make"),
        Duration::from_secs(5),
    )
    .await
    .map_err(|e| format!("turn timed out: {e}"))?
    .map_err(|e| format!("{e:?}"))?;
    assert_eq!(outcome.reason, TurnEndReason::Answered);
    let kinds = task_kinds(&world, &outcome.task);
    let shown = kinds
        .iter()
        .filter(|k| matches!(k, KnownTaskEventKind::CardShown { .. }))
        .count();
    let swapped = kinds
        .iter()
        .filter(|k| matches!(k, KnownTaskEventKind::ModelSwapped { .. }))
        .count();
    assert_eq!(shown, 0);
    assert_eq!(swapped, 0);
    assert_eq!(world.chat_requests().len(), 2);

    assert_user_always_and_turn_started(&kinds)
}

fn install_mid_step_always_hook(world: &Arc<TurnWorld>) {
    let shared = Arc::clone(world);
    let done = Arc::new(std::sync::atomic::AtomicBool::new(false));
    world.record.set_after_append(Some(Arc::new(move |row| {
        if let StoredBody::Task { event, .. } = &row.body
            && matches!(
                event.kind,
                anybot_protocol::TaskEventKind::Known(KnownTaskEventKind::ToolStarted { .. })
            )
            && !done.swap(true, std::sync::atomic::Ordering::SeqCst)
        {
            let mut updated = shared.gate_config();
            let entry = AlwaysEntry {
                action: "command make on bot-computer".to_owned(),
                added_at: anybot_foundation::TimestampMs::from_millis(shared.clock.now_ms()),
                from_bot: shared.bot,
            };
            updated.user.always.add(entry);
            shared.set_gate(updated);
        }
    })));
}

fn assert_user_always_and_turn_started(kinds: &[KnownTaskEventKind]) -> Result<(), String> {
    let started = kinds.iter().find_map(|k| {
        let KnownTaskEventKind::TurnStarted {
            model, thinking, ..
        } = k
        else {
            return None;
        };
        Some((model, *thinking))
    });
    let (model, thinking) = started.ok_or("no TurnStarted")?;
    assert_eq!(model.model, "gpt-5");
    assert_eq!(thinking, ThinkingChoice::Auto);

    let make_gate = kinds
        .iter()
        .filter_map(|k| {
            let KnownTaskEventKind::GateResult { gate, .. } = k else {
                return None;
            };
            Some(gate)
        })
        .nth(1)
        .ok_or("missing second GateResult")?;
    assert_eq!(make_gate.outcome, GateOutcome::Allow);
    assert_eq!(make_gate.layer, RuleLayer::User);
    assert_eq!(make_gate.rule, "user-rule");
    Ok(())
}

fn count_tool_messages(body: &serde_json::Value) -> usize {
    body.get("messages")
        .and_then(|m| m.as_array())
        .map_or(0, |items| {
            items
                .iter()
                .filter(|item| item.get("role").and_then(|r| r.as_str()) == Some("tool"))
                .count()
        })
}
