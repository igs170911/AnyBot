use anybot_context as _;
use anybot_domain as _;
use anybot_messages as _;
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
use runner_test::fake_scripts::CommandScript;
use runner_test::turn::{Device, TurnWorld, WorldSetup};
use runner_test::turn_helpers::{fmt_text, messages_of, request_body, sha256_hex, task_of};
use runner_test::turn_tool_helpers::{
    all_specs, call, edit_call, evidence_record, exit_evidence, intent_call_ids, one_tool_step,
    tool_message_content, world_with,
};

#[tokio::test]
async fn t_computer_ops_2() -> Result<(), String> {
    let world = world_with(Vec::new()).await?;
    world.script(one_tool_step(vec![edit_call(
        "{\"path\":\"a.txt\",\"old_text\":\"\",\"new_text\":\"x\"}",
        "c1",
    )]));
    let outcome = world.send(Device::Phone, "go").await;
    let task = task_of(&outcome.map_err(|e| format!("{e:?}"))?);
    let record = evidence_record(&world, &task, "a.txt")?;
    let anybot_protocol::EvidencePayload::Changed { before, after, .. } = &record.payload else {
        return Err("not a changed payload".to_owned());
    };
    if !matches!(before, anybot_protocol::BeforeDigest::Absent) {
        return Err("before is not absent".to_owned());
    }
    let wanted = sha256_hex(b"x");
    if after.hex() != wanted {
        return Err(format!("after {} wanted {}", after.hex(), wanted));
    }
    Ok(())
}

#[tokio::test]
async fn t_files_shell_4() -> Result<(), String> {
    let setup = WorldSetup {
        tools: all_specs(),
        scripts: vec![(
            "echo oops; exit 2".to_owned(),
            CommandScript::failing("oops\n", 2),
        )],
        ..WorldSetup::default()
    };
    let world = TurnWorld::start(&setup).await?;
    world.script(one_tool_step(vec![call(
        "shell",
        "{\"line\":\"echo oops; exit 2\"}",
        "c1",
    )]));
    let outcome = world.send(Device::Phone, "go").await;
    let task = task_of(&outcome.map_err(|e| format!("{e:?}"))?);
    let record = exit_evidence(&world, &task)?;
    let anybot_protocol::EvidencePayload::Executed {
        exit_code: Some(2), ..
    } = &record.payload
    else {
        return Err("exit code is not 2".to_owned());
    };
    check_model_sees_exit_and_output(&world, &task)?;
    Ok(())
}

fn check_model_sees_exit_and_output(
    world: &TurnWorld,
    task: &anybot_foundation::TaskId,
) -> Result<(), String> {
    let body = request_body(&world.model, 1)?;
    let ids = intent_call_ids(world, task);
    let content = tool_message_content(
        &messages_of(&body),
        ids.first()
            .map(String::as_str)
            .ok_or("no first id".to_owned())?,
    )?;
    let wanted_code = fmt_text(
        Lang::En,
        "tool-shell-failed",
        &Params::new().with_text("code", "2"),
    )?;
    if !content.contains(&wanted_code) {
        return Err(format!("no exit text in {content}"));
    }
    if !content.contains("oops") {
        return Err(format!("no output text in {content}"));
    }
    Ok(())
}
