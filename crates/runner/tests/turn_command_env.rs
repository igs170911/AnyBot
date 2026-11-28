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
use thiserror as _;
use tokio as _;

use anybot_messages::{Lang, Params};
use std::sync::Arc;
use std::time::Duration;

use anybot_protocol::{
    CapState, CardAnswer, ComputerCaps, ComputerOwner, ComputerPlatform, EvidencePayload,
    KnownTaskEventKind,
};
use runner_test::fake_scripts::CommandScript;
use runner_test::turn::{Device, TurnWorld, WorldSetup};
use runner_test::turn_helpers::{
    fmt_text, messages_of, request_body, task_kinds, task_of, wait_for_shown_card,
};
use runner_test::turn_tool_helpers::{
    call, exit_evidence, intent_call_ids, one_tool_step, tool_message_content,
};
use test_harness::timeout_real;

fn linux_caps() -> ComputerCaps {
    ComputerCaps {
        run_commands: CapState::Supported,
        command_sandbox: CapState::Supported,
        ..ComputerCaps::declared(ComputerPlatform::Linux, ComputerOwner::User)
    }
}

fn last_tool_result(
    world: &TurnWorld,
    task: &anybot_foundation::TaskId,
    call_id: &str,
) -> Result<String, String> {
    let mut found = None;
    for kind in &task_kinds(world, task) {
        if let KnownTaskEventKind::ToolCompleted {
            call_id: id,
            summary,
            ..
        } = kind
            && id.as_str() == call_id
        {
            found = Some(summary.clone());
        } else if let KnownTaskEventKind::ToolFailed {
            call_id: id, error, ..
        } = kind
            && id.as_str() == call_id
        {
            found = Some(error.clone());
        }
    }
    found.ok_or_else(|| "no tool result recorded".to_owned())
}

async fn send_allowing_card(world: &Arc<TurnWorld>) -> Result<anybot_runner::TurnOutcome, String> {
    let shared = runner_test::turn::share(world);
    let mut bg = tokio::spawn(async move { shared.send(Device::Phone, "run").await });
    tokio::select! {
        joined = &mut bg => {
            return joined
                .map_err(|e| format!("join failed: {e}"))?
                .map_err(|e| format!("{e:?}"));
        }
        shown = wait_for_shown_card(world, 1) => {
            let (task, _cards) = shown?;
            world.answer_shown_card(task, CardAnswer::AllowOnce).await?;
        }
    }
    timeout_real(bg, Duration::from_secs(5))
        .await
        .map_err(|e| format!("turn timed out: {e}"))?
        .map_err(|e| format!("join failed: {e}"))?
        .map_err(|e| format!("{e:?}"))
}

async fn run_command_and_allow(
    setup: &WorldSetup,
    cmd: &str,
) -> Result<(Arc<TurnWorld>, anybot_foundation::TaskId, String, String), String> {
    let world = runner_test::turn::arc_world(TurnWorld::start(setup).await?);
    let args = serde_json::json!({ "command": cmd }).to_string();
    world.script(one_tool_step(vec![call("shell", &args, "c1")]));
    let outcome = send_allowing_card(&world).await?;
    let task = task_of(&outcome);
    let ids = intent_call_ids(&world, &task);
    let call_id = ids.first().ok_or("no call id")?.clone();
    let result = last_tool_result(&world, &task, &call_id)?;
    Ok((world, task, call_id, result))
}

#[tokio::test]
async fn t_files_shell_6() -> Result<(), String> {
    let cmd = "echo [$ANTHROPIC_API_KEY]";
    let setup = WorldSetup {
        platform: ComputerPlatform::Linux,
        computer_name: "Linux".to_owned(),
        caps: Some(linux_caps()),
        inherited_env: vec![
            ("ANTHROPIC_API_KEY".to_owned(), "sk-test-1".to_owned()),
            ("PATH".to_owned(), "/usr/bin".to_owned()),
        ],
        scripts: vec![(
            cmd.to_owned(),
            CommandScript::ok_expanding("[$ANTHROPIC_API_KEY]"),
        )],
        ..WorldSetup::default()
    };
    let (world, _task, _call_id, result) = run_command_and_allow(&setup, cmd).await?;
    assert_eq!(result, "[]");
    let seen = world.gated.env_seen(cmd);
    assert!(!seen.contains(&"ANTHROPIC_API_KEY".to_owned()));
    assert!(seen.contains(&"PATH".to_owned()));
    Ok(())
}

fn check_executed_evidence(
    world: &TurnWorld,
    task: &anybot_foundation::TaskId,
    expected_cmd: &str,
    expected_computer: &str,
) -> Result<(), String> {
    let record = exit_evidence(world, task)?;
    let EvidencePayload::Executed {
        command,
        computer,
        exit_code,
        elapsed_ms,
        ..
    } = record.payload
    else {
        return Err("expected executed evidence".to_owned());
    };
    assert_eq!(command, expected_cmd);
    assert_eq!(computer, expected_computer);
    assert_eq!(exit_code, Some(3));
    assert_eq!(elapsed_ms, 1_200);
    Ok(())
}

#[tokio::test]
async fn t_computer_ops_1() -> Result<(), String> {
    let cmd = "echo hi; exit 3";
    let script = CommandScript::failing("hi\n", 3).with_duration(1_200);
    let bot_setup = WorldSetup {
        platform: ComputerPlatform::BotMachine,
        computer_name: "bot-computer".to_owned(),
        scripts: vec![(cmd.to_owned(), script.clone())],
        ..WorldSetup::default()
    };
    let linux_setup = WorldSetup {
        platform: ComputerPlatform::Linux,
        computer_name: "Linux".to_owned(),
        caps: Some(linux_caps()),
        scripts: vec![(cmd.to_owned(), script)],
        ..WorldSetup::default()
    };
    let (bot_world, bot_task, bot_call_id, bot_result) =
        run_command_and_allow(&bot_setup, cmd).await?;
    let bot_model_msg = tool_message_content(
        &messages_of(&request_body(&bot_world.model, 1)?),
        &bot_call_id,
    )?;
    assert_eq!(bot_model_msg, bot_result);
    let (linux_world, linux_task, _linux_call_id, linux_result) =
        run_command_and_allow(&linux_setup, cmd).await?;
    assert_eq!(bot_result, linux_result);
    let failed_line = fmt_text(
        Lang::En,
        "tool-shell-failed",
        &Params::new().with_text("code", "3"),
    )?;
    assert!(bot_result.contains("hi"), "got: {bot_result}");
    assert!(bot_result.contains(&failed_line), "got: {bot_result}");
    check_executed_evidence(&bot_world, &bot_task, cmd, "bot-computer")?;
    check_executed_evidence(&linux_world, &linux_task, cmd, "Linux")?;
    Ok(())
}
