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

use anybot_foundation::CallId;
use anybot_protocol::{
    BudgetItem, KnownTaskEventKind, NotRunReason, TaskDetail, TaskStatus, TurnEndReason,
};
use runner_test::turn::{Device, TurnWorld, WorldSetup};
use runner_test::turn_helpers::{chat_requests, state_of, task_kinds, task_of};
use runner_test::turn_tool_helpers::{all_specs, call, edit_call, one_tool_step};
use test_harness::{
    Step, UsageScript, model_replies_text, model_tool_calls, model_tool_calls_with_usage,
};

fn read_call(path: &str, id: &str) -> test_harness::ToolCallScript {
    call("read_file", &format!("{{\"path\":\"{path}\"}}"), id)
}

#[tokio::test]
async fn t_turn_loop_6() -> Result<(), String> {
    let setup = WorldSetup {
        tools: all_specs(),
        files: vec![
            ("a.txt".to_owned(), "alpha".to_owned()),
            ("b.txt".to_owned(), "beta".to_owned()),
            ("c.txt".to_owned(), "gamma".to_owned()),
        ],
        budget_overrides: vec![(BudgetItem::ToolCalls, 2)],
        ..WorldSetup::default()
    };
    let world = TurnWorld::start(&setup).await?;
    world.script(one_tool_step(vec![
        read_call("a.txt", "c1"),
        read_call("b.txt", "c2"),
        read_call("c.txt", "c3"),
    ]));
    let outcome = world
        .send(Device::Phone, "read three files")
        .await
        .map_err(|e| format!("{e:?}"))?;
    let task = task_of(&outcome);
    assert_eq!(
        outcome.reason,
        TurnEndReason::OverBudget {
            item: BudgetItem::ToolCalls,
            used: 2,
            limit: 2,
        }
    );
    assert_eq!(chat_requests(&world.model).len(), 1);
    check_two_completed_third_over_budget(&task_kinds(&world, &task))?;
    assert_eq!(
        state_of(&world, &task)?,
        (
            TaskStatus::Stuck,
            Some(TaskDetail::OverBudget {
                item: BudgetItem::ToolCalls,
                used: 2,
                limit: 2,
            })
        )
    );
    Ok(())
}

fn check_two_completed_third_over_budget(kinds: &[KnownTaskEventKind]) -> Result<(), String> {
    let requests: Vec<CallId> = kinds
        .iter()
        .find_map(|k| {
            let KnownTaskEventKind::ModelAnswer { tool_requests, .. } = k else {
                return None;
            };
            Some(
                tool_requests
                    .iter()
                    .map(|req| req.call_id)
                    .collect::<Vec<CallId>>(),
            )
        })
        .ok_or_else(|| "missing model_answer".to_owned())?;
    if requests.len() != 3 {
        return Err(format!("expected 3 requests, got {}", requests.len()));
    }
    let completed: Vec<CallId> = kinds
        .iter()
        .filter_map(|k| {
            let KnownTaskEventKind::ToolCompleted { call_id, .. } = k else {
                return None;
            };
            Some(*call_id)
        })
        .collect();
    assert_eq!(completed.as_slice(), requests.get(..2).unwrap_or(&[]));
    let not_run: Vec<(CallId, NotRunReason)> = kinds
        .iter()
        .filter_map(|k| {
            let KnownTaskEventKind::ToolNotRun { call_id, reason } = k else {
                return None;
            };
            Some((*call_id, *reason))
        })
        .collect();
    let third = requests
        .get(2)
        .copied()
        .ok_or_else(|| "no third".to_owned())?;
    assert_eq!(not_run, vec![(third, NotRunReason::OverBudget)]);
    Ok(())
}

#[tokio::test]
async fn t_turn_loop_7() -> Result<(), String> {
    let setup = WorldSetup {
        tools: all_specs(),
        files: vec![
            ("a.txt".to_owned(), "alpha".to_owned()),
            ("b.txt".to_owned(), "beta".to_owned()),
        ],
        budget_overrides: vec![(BudgetItem::ToolCalls, 2)],
        ..WorldSetup::default()
    };
    let world = TurnWorld::start(&setup).await?;
    world.script(vec![
        model_tool_calls(vec![read_call("a.txt", "c1")], "tool_calls", false),
        model_tool_calls(vec![read_call("b.txt", "c2")], "tool_calls", false),
        model_replies_text("done", false),
    ]);
    let outcome = world
        .send(Device::Phone, "read two files")
        .await
        .map_err(|e| format!("{e:?}"))?;
    let task = task_of(&outcome);
    assert_eq!(outcome.reason, TurnEndReason::Answered);
    assert_eq!(chat_requests(&world.model).len(), 3);
    assert_eq!(
        state_of(&world, &task)?,
        (TaskStatus::Completed, Some(TaskDetail::Accepted))
    );
    Ok(())
}

#[tokio::test]
async fn t_turn_loop_8() -> Result<(), String> {
    let setup = WorldSetup {
        tools: all_specs(),
        budget_overrides: vec![(BudgetItem::Tokens, 1_000)],
        ..WorldSetup::default()
    };
    let world = TurnWorld::start(&setup).await?;
    world.script(vec![model_tool_calls_with_usage(
        vec![edit_call(
            "{\"path\":\"a.txt\",\"old_text\":\"\",\"new_text\":\"x\"}",
            "c1",
        )],
        UsageScript::plain(900, 300),
        false,
    )]);
    let outcome = world
        .send(Device::Phone, "write a.txt")
        .await
        .map_err(|e| format!("{e:?}"))?;
    let task = task_of(&outcome);
    assert_eq!(chat_requests(&world.model).len(), 1);
    assert_eq!(world.tree_content("a.txt"), None);
    assert_eq!(
        outcome.reason,
        TurnEndReason::OverBudget {
            item: BudgetItem::Tokens,
            used: 1_200,
            limit: 1_000,
        }
    );
    check_answer_kept_and_edit_not_run(&task_kinds(&world, &task))?;
    assert_eq!(
        state_of(&world, &task)?,
        (
            TaskStatus::Stuck,
            Some(TaskDetail::OverBudget {
                item: BudgetItem::Tokens,
                used: 1_200,
                limit: 1_000,
            })
        )
    );
    Ok(())
}

fn check_answer_kept_and_edit_not_run(kinds: &[KnownTaskEventKind]) -> Result<(), String> {
    let call_id = kinds
        .iter()
        .find_map(|k| {
            let KnownTaskEventKind::ModelAnswer { tool_requests, .. } = k else {
                return None;
            };
            tool_requests
                .first()
                .filter(|req| req.tool == "edit_file")
                .map(|req| req.call_id)
        })
        .ok_or_else(|| "edit_file missing from model_answer".to_owned())?;
    let not_run: Vec<(CallId, NotRunReason)> = kinds
        .iter()
        .filter_map(|k| {
            let KnownTaskEventKind::ToolNotRun { call_id, reason } = k else {
                return None;
            };
            Some((*call_id, *reason))
        })
        .collect();
    assert_eq!(not_run, vec![(call_id, NotRunReason::OverBudget)]);
    Ok(())
}

#[tokio::test]
async fn default_steps_stop_before_301st_request() -> Result<(), String> {
    let setup = WorldSetup {
        tools: all_specs(),
        files: vec![("a.txt".to_owned(), "alpha".to_owned())],
        ..WorldSetup::default()
    };
    let world = TurnWorld::start(&setup).await?;
    let mut script: Vec<Step> = (1..=305u32)
        .map(|i| {
            model_tool_calls(
                vec![read_call("a.txt", &format!("c{i}"))],
                "tool_calls",
                false,
            )
        })
        .collect();
    script.push(model_replies_text("never reached", false));
    world.script(script);
    let outcome = world
        .send(Device::Phone, "keep reading")
        .await
        .map_err(|e| format!("{e:?}"))?;
    let task = task_of(&outcome);
    assert_eq!(chat_requests(&world.model).len(), 300);
    assert_eq!(
        outcome.reason,
        TurnEndReason::OverBudget {
            item: BudgetItem::Steps,
            used: 300,
            limit: 300,
        }
    );
    assert_eq!(
        state_of(&world, &task)?,
        (
            TaskStatus::Stuck,
            Some(TaskDetail::OverBudget {
                item: BudgetItem::Steps,
                used: 300,
                limit: 300,
            })
        )
    );
    Ok(())
}
