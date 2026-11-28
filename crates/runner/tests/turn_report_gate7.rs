use anybot_context as _;
use anybot_domain as _;
use anybot_messages as _;
use anybot_models as _;
use anybot_runner as _;
use anybot_store_client as _;
use anybot_tools as _;
use anybot_trust as _;
use proptest as _;
use serde_json as _;
use sha2 as _;
use thiserror as _;

use anybot_protocol::{TaskDetail, TaskStatus};
use runner_test::fake_scripts::CommandScript;
use runner_test::turn::{Device, TurnWorld, WorldSetup};
use runner_test::turn_helpers::{delivery_of, task_of};
use runner_test::turn_tool_helpers::{call, edit_call};
use test_harness::{Step, model_replies_text, model_tool_calls};

#[tokio::test]
async fn t_task_done_gate_7() -> Result<(), String> {
    let setup = gate7_setup();
    let world = TurnWorld::start(&setup).await?;
    world.script(gate7_script());
    let outcome = world.send(Device::Desktop, "make x exist").await;
    let task = task_of(&outcome.map_err(|e| format!("{e:?}"))?);
    let state = world.task_state(task)?;
    if state.0 != TaskStatus::Completed || state.1 != Some(TaskDetail::Accepted) {
        return Err(format!("state {state:?}"));
    }
    check_gate7_attention(&world, task)
}

fn gate7_setup() -> WorldSetup {
    WorldSetup {
        scripts: vec![("test -f x".to_owned(), CommandScript::failing("", 1))],
        ..WorldSetup::default()
    }
}

fn gate7_script() -> Vec<Step> {
    vec![
        model_tool_calls(
            vec![call("shell", "{\"command\":\"test -f x\"}", "c1")],
            "tool_calls",
            false,
        ),
        model_tool_calls(
            vec![edit_call(
                "{\"path\":\"x\",\"old_text\":\"\",\"new_text\":\"1\"}",
                "c2",
            )],
            "tool_calls",
            false,
        ),
        model_replies_text("good", true),
    ]
}

fn check_gate7_attention(world: &TurnWorld, task: anybot_foundation::TaskId) -> Result<(), String> {
    let report = delivery_of(world, task)?;
    if report.attention.is_empty() {
        return Err("no attention entries".to_owned());
    }
    let Some(entry) = report.attention.first() else {
        return Err("no first attention entry".to_owned());
    };
    if entry.tool != "shell" || !entry.args_summary.contains("test -f x") {
        return Err(format!("attention {entry:?}"));
    }
    Ok(())
}
