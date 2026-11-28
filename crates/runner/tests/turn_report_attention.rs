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

use anybot_protocol::AttentionKind;
use anybot_trust::{ActionKey, GateConfig};
use runner_test::fake_scripts::CommandScript;
use runner_test::turn::{Device, TurnWorld, WorldSetup};
use runner_test::turn_helpers::{delivery_of, task_of};
use runner_test::turn_tool_helpers::{call, edit_call, one_tool_step};

const NPM: &str = "npm install";

fn npm_args() -> String {
    format!("{{\"line\":\"{NPM}\"}}")
}

fn world_setup(script: CommandScript) -> WorldSetup {
    let mut gate = GateConfig::plain();
    gate.user.ask_style.push(ActionKey::command("shell", NPM));
    WorldSetup {
        scripts: vec![(NPM.to_owned(), script)],
        gate,
        ..WorldSetup::default()
    }
}

#[tokio::test]
async fn rejected_action_listed_in_attention() -> Result<(), String> {
    let setup = WorldSetup {
        ..WorldSetup::default()
    };
    let world = TurnWorld::start(&setup).await?;
    world.script(one_tool_step(vec![
        edit_call(
            "{\"path\":\"a.txt\",\"old_text\":\"\",\"new_text\":\"1\"}",
            "c1",
        ),
        call("shell", "{\"line\":\"sudo rm -rf /\"}", "c2"),
    ]));
    let outcome = world
        .send(Device::Phone, "work")
        .await
        .map_err(|e| format!("{e:?}"))?;
    let report = delivery_of(&world, task_of(&outcome))?;
    let rejected: Vec<_> = report
        .attention
        .iter()
        .filter(|entry| entry.kind == AttentionKind::Rejected)
        .collect();
    assert_eq!(rejected.len(), 1);
    let entry = rejected.first().ok_or("no rejected entry")?;
    assert_eq!(entry.tool, "shell");
    assert!(entry.args_summary.contains("sudo rm"));
    Ok(())
}

#[tokio::test]
async fn failed_action_listed_in_attention() -> Result<(), String> {
    let world = TurnWorld::start(&world_setup(CommandScript::failing("boom", 3))).await?;
    let mut gate = GateConfig::plain();
    gate.user.ask_style.clear();
    world.set_gate(gate);
    world.script(one_tool_step(vec![call("shell", &npm_args(), "c1")]));
    let outcome = world
        .send(Device::Phone, "install")
        .await
        .map_err(|e| format!("{e:?}"))?;
    let task = task_of(&outcome);
    let report = delivery_of(&world, task)?;
    let failed: Vec<_> = report
        .attention
        .iter()
        .filter(|entry| entry.kind == AttentionKind::Failed)
        .collect();
    assert!(!failed.is_empty());
    Ok(())
}
