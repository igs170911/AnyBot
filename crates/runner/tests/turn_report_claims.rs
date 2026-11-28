use anybot_context as _;
use anybot_domain as _;
use anybot_foundation as _;
use anybot_messages as _;
use anybot_models as _;
use anybot_protocol as _;
use anybot_runner as _;
use anybot_tools as _;
use anybot_trust as _;
use proptest as _;
use serde_json as _;
use sha2 as _;
use test_harness as _;
use thiserror as _;

use anybot_messages::{Catalog, Lang, Params, TestCode};
use anybot_protocol::{ReportLevel, TaskStatus};
use anybot_trust::{CODE_CLAIM_KIND_MISMATCH, CODE_CLAIM_NOT_IN_TASK};
use runner_test::fake_scripts::CommandScript;
use runner_test::turn::{Device, TurnWorld, WorldSetup};
use runner_test::turn_helpers::{delivery_of, task_of};
use runner_test::turn_tool_helpers::{all_specs, call, edit_call};
use test_harness::{model_replies_text, model_tool_calls};

#[tokio::test]
async fn t_report_delivery_4() -> Result<(), String> {
    let setup = WorldSetup {
        tools: all_specs(),
        files: vec![("a.txt".to_owned(), "hello".to_owned())],
        scripts: vec![("echo hi".to_owned(), CommandScript::ok("hi"))],
        ..WorldSetup::default()
    };
    let world = TurnWorld::start(&setup).await?;
    world.script(vec![
        model_tool_calls(
            vec![call("shell", "{\"line\":\"echo hi\"}", "c1")],
            "tool_calls",
            false,
        ),
        model_tool_calls(
            vec![call("read_file", "{\"path\":\"a.txt\"}", "c2")],
            "tool_calls",
            false,
        ),
        model_replies_text("\u{5df2}\u{5bc4}\u{7d66} alice\u{3014}E2\u{3015}", false),
    ]);
    let outcome = world
        .send(Device::Phone, "send to alice")
        .await
        .map_err(|e| format!("{e:?}"))?;
    let task = task_of(&outcome);
    let state = world.task_state(task)?;
    assert_eq!(state.0, TaskStatus::Completed);
    assert_eq!(world.chat_requests().len(), 3);
    let report = delivery_of(&world, task)?;
    assert_eq!(report.claims.len(), 1);
    let claim = report.claims.first().ok_or("missing claim")?;
    assert_eq!(claim.level, ReportLevel::Unknown);
    assert_eq!(report.level_counts.unknown, 1);
    let reason = claim
        .reasons
        .iter()
        .find(|r| r.code == CODE_CLAIM_KIND_MISMATCH)
        .ok_or("missing claim-kind-mismatch")?;
    let mut params = Params::new();
    for (k, v) in &reason.params {
        params = params.with_text(k, v);
    }
    let mut catalog = Catalog::new();
    let rendered = catalog
        .format(Lang::ZhTw, &TestCode::new(&reason.code), &params)
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(
        rendered,
        "\u{5f15}\u{7528}\u{7684}\u{4e0d}\u{662f}\u{5bc4}\u{51fa}"
    );
    Ok(())
}

#[tokio::test]
async fn t_report_delivery_6() -> Result<(), String> {
    let setup = WorldSetup {
        tools: all_specs(),
        files: vec![("a.txt".to_owned(), "hello".to_owned())],
        scripts: vec![("echo hi".to_owned(), CommandScript::ok("hi"))],
        ..WorldSetup::default()
    };
    let world = TurnWorld::start(&setup).await?;
    let task1 = run_first_task(&world).await?;

    world.script(vec![
        model_tool_calls(
            vec![edit_call(
                "{\"path\":\"b.txt\",\"old_text\":\"\",\"new_text\":\"hi\"}",
                "c4",
            )],
            "tool_calls",
            false,
        ),
        model_replies_text("\u{5df2}\u{6539}\u{597d} b.txt\u{3014}E3\u{3015}", false),
    ]);
    let second = world
        .send(Device::Phone, "second task")
        .await
        .map_err(|e| format!("{e:?}"))?;
    let task2 = task_of(&second);
    assert_ne!(task1, task2);
    let report = delivery_of(&world, task2)?;
    assert_eq!(report.claims.len(), 1);
    let claim = report.claims.first().ok_or("missing claim")?;
    assert_eq!(claim.level, ReportLevel::Unknown);
    assert_eq!(report.level_counts.unknown, 1);
    assert!(
        claim
            .reasons
            .iter()
            .any(|r| r.code == CODE_CLAIM_NOT_IN_TASK)
    );
    let mut catalog = Catalog::new();
    let msg_not_in_task = catalog
        .format(
            Lang::ZhTw,
            &TestCode::new(CODE_CLAIM_NOT_IN_TASK),
            &Params::new(),
        )
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(
        msg_not_in_task,
        "\u{5f15}\u{7528}\u{7684}\u{8b49}\u{64da}\u{4e0d}\u{5728}\u{9019}\u{500b}\u{4efb}\u{52d9}\u{88e1}"
    );
    Ok(())
}

async fn run_first_task(world: &TurnWorld) -> Result<anybot_foundation::TaskId, String> {
    world.script(vec![
        model_tool_calls(
            vec![
                call("read_file", "{\"path\":\"a.txt\"}", "c1"),
                call("shell", "{\"line\":\"echo hi\"}", "c2"),
                edit_call(
                    "{\"path\":\"a.txt\",\"old_text\":\"hello\",\"new_text\":\"v2\"}",
                    "c3",
                ),
            ],
            "tool_calls",
            false,
        ),
        model_replies_text("first done", true),
    ]);
    let first = world
        .send(Device::Phone, "first task")
        .await
        .map_err(|e| format!("{e:?}"))?;
    Ok(task_of(&first))
}
