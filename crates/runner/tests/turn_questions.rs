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

use std::time::Duration;

use anybot_messages::{Lang, Params};
use anybot_protocol::{KnownTaskEventKind, QuestionAnswer, TaskDetail, TaskStatus, TurnEndReason};
use anybot_tools::{CODE_QUESTION_SKIPPED, CODE_QUESTION_UNANSWERED, CODE_TOOL_ASK_TOO_MANY};
use anybot_trust::{ActionKey, GateConfig};
use runner_test::turn::{Device, TurnWorld, WorldSetup, tool_spec};
use runner_test::turn_helpers::{
    fmt_text, messages_of, request_body, state_of, task_kinds, task_of,
};
use runner_test::turn_tool_helpers::{all_specs, call, intent_call_ids, tool_message_content};
use test_harness::{advance_clock, model_replies_text, model_tool_calls};

fn ask_tools() -> Vec<anybot_models::ToolSpec> {
    let mut specs = all_specs();
    specs.push(tool_spec("ask_person", "Ask the person questions"));
    specs
}

fn one_question_call(title: &str, id: &str) -> test_harness::ToolCallScript {
    let args = serde_json::json!({
        "questions": [{
            "title": title,
            "kind": "single",
            "options": ["A", "B"]
        }]
    })
    .to_string();
    call("ask_person", &args, id)
}

fn first_question_card(
    world: &TurnWorld,
    task: &anybot_foundation::TaskId,
) -> Result<String, String> {
    for kind in task_kinds(world, task) {
        if let KnownTaskEventKind::QuestionShown { card, .. } = kind {
            return Ok(card);
        }
    }
    Err("missing QuestionShown".to_owned())
}

#[tokio::test]
async fn t_cards_question_5() -> Result<(), String> {
    let setup = WorldSetup {
        tools: ask_tools(),
        ..WorldSetup::default()
    };
    let world = TurnWorld::start(&setup).await?;
    world.script(vec![model_tool_calls(
        vec![one_question_call("Pick one", "c1")],
        "tool_calls",
        false,
    )]);
    let parked = world
        .send(Device::Phone, "ask me")
        .await
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(parked.reason, TurnEndReason::PendingApproval);
    let task = task_of(&parked);
    let card = first_question_card(&world, &task)?;

    world.script(vec![model_replies_text("ok, continuing", false)]);
    let ctx = world.runner().approval_context(task, parked.turn);
    let resumed = world
        .runner()
        .answer_question(&ctx, &card, QuestionAnswer::Skipped)
        .await
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(resumed.reason, TurnEndReason::Answered);
    assert!(task_kinds(&world, &task).iter().any(|k| {
        matches!(
            k,
            KnownTaskEventKind::QuestionAnswered {
                card: c,
                answer: QuestionAnswer::Skipped
            } if c == &card
        )
    }));
    let body = request_body(&world.model, 1)?;
    let ids = intent_call_ids(&world, &task);
    let first_id = ids.first().map(String::as_str).ok_or("missing call_id")?;
    let tool_msg = tool_message_content(&messages_of(&body), first_id)?;
    let wanted = fmt_text(Lang::En, CODE_QUESTION_SKIPPED, &Params::new())?;
    assert_eq!(tool_msg, wanted);
    let state = state_of(&world, &task)?;
    assert_eq!(state.0, TaskStatus::Completed);
    let duplicate = world
        .runner()
        .answer_question(&ctx, &card, QuestionAnswer::Skipped)
        .await;
    assert!(duplicate.is_err());
    Ok(())
}

#[tokio::test]
async fn t_cards_question_6() -> Result<(), String> {
    let setup = WorldSetup {
        tools: ask_tools(),
        ..WorldSetup::default()
    };
    let world = TurnWorld::start(&setup).await?;
    world.script(vec![model_tool_calls(
        vec![one_question_call("Pick one", "c1")],
        "tool_calls",
        false,
    )]);
    let parked = world
        .send(Device::Phone, "ask me")
        .await
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(parked.reason, TurnEndReason::PendingApproval);
    let task = task_of(&parked);
    let card = first_question_card(&world, &task)?;

    world.script(vec![model_replies_text("whatever works", false)]);
    let casual = "\u{96a8}\u{4fbf}";
    let resumed = world
        .send(Device::Phone, casual)
        .await
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(resumed.task, task);
    assert_eq!(resumed.reason, TurnEndReason::Answered);
    assert!(task_kinds(&world, &task).iter().any(|k| {
        matches!(
            k,
            KnownTaskEventKind::QuestionAnswered {
                card: c,
                answer: QuestionAnswer::Unanswered
            } if c == &card
        )
    }));
    let body = request_body(&world.model, 1)?;
    let body_json = serde_json::to_string(&body).map_err(|e| format!("{e}"))?;
    assert!(body_json.contains(casual));
    let ids = intent_call_ids(&world, &task);
    let first_id = ids.first().map(String::as_str).ok_or("missing call_id")?;
    let tool_msg = tool_message_content(&messages_of(&body), first_id)?;
    let wanted = fmt_text(Lang::En, CODE_QUESTION_UNANSWERED, &Params::new())?;
    assert_eq!(tool_msg, wanted);
    Ok(())
}

#[tokio::test]
async fn t_cards_question_11() -> Result<(), String> {
    let mut gate = GateConfig::plain();
    gate.modes.yolo = true;
    gate.modes.direct_mode = true;
    gate.user
        .session
        .push(ActionKey::capabilities("ask_person"));
    let setup = WorldSetup {
        tools: ask_tools(),
        gate,
        ..WorldSetup::default()
    };
    let world = TurnWorld::start(&setup).await?;
    world.script(vec![model_tool_calls(
        vec![one_question_call("Confirm branch", "c1")],
        "tool_calls",
        false,
    )]);
    let outcome = world
        .send(Device::Phone, "go")
        .await
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(outcome.reason, TurnEndReason::PendingApproval);
    let _advanced = advance_clock(&world.clock, Duration::from_secs(600));
    let task = task_of(&outcome);
    let kinds = task_kinds(&world, &task);
    assert!(
        kinds
            .iter()
            .any(|k| matches!(k, KnownTaskEventKind::QuestionShown { .. }))
    );
    assert!(
        !kinds
            .iter()
            .any(|k| matches!(k, KnownTaskEventKind::QuestionAnswered { .. }))
    );
    let state = state_of(&world, &task)?;
    assert_eq!(state.0, TaskStatus::AwaitingUser);
    assert!(matches!(state.1, Some(TaskDetail::QuestionPending { .. })));
    Ok(())
}

#[tokio::test]
async fn ask_person_limits_and_truncation() -> Result<(), String> {
    let setup = WorldSetup {
        tools: ask_tools(),
        ..WorldSetup::default()
    };
    let world = TurnWorld::start(&setup).await?;
    let seven = serde_json::json!({
        "questions": (0..7)
            .map(|i| serde_json::json!({"title": format!("Q{i}"), "kind": "text"}))
            .collect::<Vec<_>>()
    })
    .to_string();
    let long_title = "x".repeat(301);
    world.script(vec![
        model_tool_calls(vec![call("ask_person", &seven, "c1")], "tool_calls", false),
        model_tool_calls(
            vec![one_question_call(&long_title, "c2")],
            "tool_calls",
            false,
        ),
    ]);
    let outcome = world
        .send(Device::Phone, "ask")
        .await
        .map_err(|e| format!("{e:?}"))?;
    let task = task_of(&outcome);
    let kinds = task_kinds(&world, &task);
    assert!(kinds.iter().any(|k| {
        matches!(k, KnownTaskEventKind::ToolFailed { error, .. } if error == CODE_TOOL_ASK_TOO_MANY)
    }));
    let shown_titles: Vec<usize> = kinds
        .iter()
        .filter_map(|k| {
            let KnownTaskEventKind::QuestionShown { questions, .. } = k else {
                return None;
            };
            questions.first().map(|q| q.title.chars().count())
        })
        .collect();
    assert_eq!(shown_titles, vec![300]);
    Ok(())
}

#[tokio::test]
async fn question_multi_answers_join_with_dunhao() -> Result<(), String> {
    let setup = WorldSetup {
        tools: ask_tools(),
        ..WorldSetup::default()
    };
    let world = TurnWorld::start(&setup).await?;
    world.script(vec![model_tool_calls(
        vec![one_question_call("Pick", "c1")],
        "tool_calls",
        false,
    )]);
    let parked = world
        .send(Device::Phone, "ask")
        .await
        .map_err(|e| format!("{e:?}"))?;
    let task = task_of(&parked);
    let card = first_question_card(&world, &task)?;
    world.script(vec![model_replies_text("done", false)]);
    let ctx = world.runner().approval_context(task, parked.turn);
    let _resumed = world
        .runner()
        .answer_question(
            &ctx,
            &card,
            QuestionAnswer::Answers(vec!["\u{7532}".to_owned(), "\u{4e59}".to_owned()]),
        )
        .await
        .map_err(|e| format!("{e:?}"))?;
    let body = request_body(&world.model, 1)?;
    let ids = intent_call_ids(&world, &task);
    let first_id = ids.first().map(String::as_str).ok_or("missing call_id")?;
    let tool_msg = tool_message_content(&messages_of(&body), first_id)?;
    assert_eq!(tool_msg, "\u{7532}\u{3001}\u{4e59}");
    Ok(())
}
