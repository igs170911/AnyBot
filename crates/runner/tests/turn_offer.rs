use anybot_context as _;
use anybot_domain as _;
use anybot_foundation as _;
use anybot_models as _;
use anybot_runner as _;
use anybot_tools as _;
use anybot_trust as _;
use proptest as _;
use sha2 as _;
use thiserror as _;
use tokio as _;

use anybot_messages::{Lang, Params};
use anybot_protocol::{
    CapState, ComputerCaps, ComputerOwner, ComputerPlatform, EvidencePayload, KnownTaskEventKind,
};
use runner_test::fake_builder::FakeComputerBuilder;
use runner_test::turn::{Device, TurnWorld, WorldSetup};
use runner_test::turn_helpers::{
    fmt_text, messages_of, request_body, system_content, task_kinds, task_of,
};
use runner_test::turn_tool_helpers::{call, edit_call, one_tool_step, tool_message_content};
use test_harness::model_replies_text;

#[tokio::test]
async fn t_computer_platforms_1() -> Result<(), String> {
    let caps = ComputerCaps {
        read_files: CapState::Supported,
        write_files: CapState::Supported,
        run_commands: CapState::Supported,
        check_path: CapState::Supported,
        command_sandbox: CapState::Supported,
        ..ComputerCaps::declared(ComputerPlatform::BotMachine, ComputerOwner::Bot)
    };
    let setup = WorldSetup {
        caps: Some(caps),
        ..WorldSetup::default()
    };
    let world = TurnWorld::start(&setup).await?;
    world.script(vec![model_replies_text("ok", false)]);
    let outcome = world.send(Device::Phone, "hi").await;
    drop(outcome.map_err(|e| format!("{e:?}"))?);
    let body = request_body(&world.model, 0)?;
    let tools = body
        .get("tools")
        .and_then(|v| v.as_array())
        .ok_or("missing tools array".to_owned())?;
    let names = offered_names(tools)?;
    assert_eq!(names, vec!["edit_file", "read_file", "shell"]);
    assert!(!names.iter().any(|n| n == "browser" || n == "computer"));
    for tool in tools {
        let props = tool
            .get("function")
            .and_then(|f| f.get("parameters"))
            .and_then(|p| p.get("properties"));
        assert!(props.and_then(|p| p.get("computer")).is_none());
    }
    Ok(())
}

fn offered_names(tools: &[serde_json::Value]) -> Result<Vec<String>, String> {
    let mut names = Vec::new();
    for tool in tools {
        let name = tool
            .get("function")
            .and_then(|f| f.get("name"))
            .and_then(|n| n.as_str())
            .ok_or("tool missing function.name".to_owned())?;
        names.push(name.to_owned());
    }
    Ok(names)
}

#[tokio::test]
async fn t_computer_route_4() -> Result<(), String> {
    let other = FakeComputerBuilder::default()
        .named("bot-computer")
        .bot_machine()
        .build();
    let setup = WorldSetup {
        platform: ComputerPlatform::Linux,
        computer_name: "Linux".to_owned(),
        ..WorldSetup::default()
    };
    let world = TurnWorld::start(&setup).await?;
    world.script(one_tool_step(vec![call(
        "shell",
        "{\"command\":\"ls\",\"computer\":\"bot-computer\"}",
        "c1",
    )]));
    let outcome = world.send(Device::Phone, "run on bot").await;
    let task = task_of(&outcome.map_err(|e| format!("{e:?}"))?);
    let call_id = first_failed_call_id(&world, &task)?;
    let body = request_body(&world.model, 1)?;
    let got = tool_message_content(&messages_of(&body), &call_id)?;
    let wanted = fmt_text(
        Lang::En,
        "tool-other-computer",
        &Params::new().with_text("computer", "Linux"),
    )?;
    assert_eq!(got, wanted);
    assert!(other.logged().is_empty());
    assert!(!world.gated.logged().iter().any(|line| line.contains("ls")));
    Ok(())
}

fn first_failed_call_id(
    world: &TurnWorld,
    task: &anybot_foundation::TaskId,
) -> Result<String, String> {
    for kind in task_kinds(world, task) {
        if let KnownTaskEventKind::ToolFailed { call_id, .. } = kind {
            return Ok(call_id.as_str());
        }
    }
    Err("no tool_failed event".to_owned())
}

#[tokio::test]
async fn t_files_n2_6() -> Result<(), String> {
    let bot_setup = WorldSetup {
        lang: Some(Lang::ZhTw),
        platform: ComputerPlatform::BotMachine,
        ..WorldSetup::default()
    };
    let linux_setup = WorldSetup {
        lang: Some(Lang::ZhTw),
        platform: ComputerPlatform::Linux,
        computer_name: "Linux".to_owned(),
        ..WorldSetup::default()
    };
    let bot_world = TurnWorld::start(&bot_setup).await?;
    let linux_world = TurnWorld::start(&linux_setup).await?;
    bot_world.script(vec![model_replies_text("ok", false)]);
    linux_world.script(vec![model_replies_text("ok", false)]);
    drop(
        bot_world
            .send(Device::Phone, "hi")
            .await
            .map_err(|e| format!("{e:?}"))?,
    );
    drop(
        linux_world
            .send(Device::Phone, "hi")
            .await
            .map_err(|e| format!("{e:?}"))?,
    );
    let bot_sys = system_content(&request_body(&bot_world.model, 0)?)?;
    let linux_sys = system_content(&request_body(&linux_world.model, 0)?)?;
    let bot_rule = fmt_text(
        Lang::ZhTw,
        "turn-fixed-computer-bot-machine",
        &Params::new(),
    )?;
    let linux_rule = fmt_text(Lang::ZhTw, "turn-fixed-computer-linux", &Params::new())?;
    assert!(bot_sys.contains(&bot_rule));
    assert!(bot_sys.contains("\u{4e0d}\u{7528}\u{554f}"));
    assert!(!bot_sys.contains(&linux_rule));
    assert!(linux_sys.contains(&linux_rule));
    assert!(linux_sys.contains("\u{5148}\u{554f}"));
    assert!(!linux_sys.contains(&bot_rule));
    Ok(())
}

#[tokio::test]
async fn unknown_tool_lists_this_turn() -> Result<(), String> {
    let caps = ComputerCaps {
        read_files: CapState::Supported,
        ..ComputerCaps::declared(ComputerPlatform::BotMachine, ComputerOwner::Bot)
    };
    let setup = WorldSetup {
        caps: Some(caps),
        ..WorldSetup::default()
    };
    let world = TurnWorld::start(&setup).await?;
    world.script(one_tool_step(vec![edit_call(
        "{\"path\":\"a.txt\",\"old_text\":\"\",\"new_text\":\"x\"}",
        "c1",
    )]));
    let outcome = world.send(Device::Phone, "write").await;
    let task = task_of(&outcome.map_err(|e| format!("{e:?}"))?);
    let call_id = first_failed_call_id(&world, &task)?;
    let body = request_body(&world.model, 1)?;
    let got = tool_message_content(&messages_of(&body), &call_id)?;
    let wanted = fmt_text(
        Lang::En,
        "tool-not-in-turn",
        &Params::new()
            .with_text("tool", "edit_file")
            .with_text("available", "read_file"),
    )?;
    assert_eq!(got, wanted);
    let changed = task_kinds(&world, &task).iter().any(|k| {
        matches!(
            k,
            KnownTaskEventKind::EvidenceAdded { record }
            if matches!(record.payload, EvidencePayload::Changed { .. })
        )
    });
    assert!(!changed);
    assert!(world.gated.logged().is_empty());
    Ok(())
}

#[tokio::test]
async fn empty_caps_still_asks_model_with_no_tools() -> Result<(), String> {
    let caps = ComputerCaps::declared(ComputerPlatform::Linux, ComputerOwner::User);
    let setup = WorldSetup {
        caps: Some(caps),
        ..WorldSetup::default()
    };
    let world = TurnWorld::start(&setup).await?;
    world.script(vec![model_replies_text("only chat", false)]);
    let outcome = world.send(Device::Phone, "hello").await;
    drop(outcome.map_err(|e| format!("{e:?}"))?);
    let body = request_body(&world.model, 0)?;
    assert!(body.get("tools").is_none());
    Ok(())
}
