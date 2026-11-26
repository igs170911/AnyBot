use anybot_foundation::{CallId, SystemClock};
use anybot_protocol::{
    BudgetItem, CardAnswer, CheckSource, KnownTaskEventKind, TaskDetail, TaskStatus, ToolEffect,
};

use crate::task_engine::advance;
use crate::task_view::{TaskView, initial_view};

fn turn_started_kind() -> KnownTaskEventKind {
    KnownTaskEventKind::TurnStarted {
        turn: 1,
        initiator: anybot_protocol::TurnInitiator::You {
            device: anybot_foundation::DeviceId::generate(&SystemClock),
        },
        computer: anybot_protocol::ComputerRef {
            name: "c".to_owned(),
            platform: anybot_protocol::ComputerPlatform::BotMachine,
        },
        model: anybot_protocol::ModelName {
            provider: "OpenAi".to_owned(),
            base_url: "http://127.0.0.1:9/v1".to_owned(),
            model: "m".to_owned(),
        },
        thinking: anybot_protocol::ThinkingChoice::Auto,
    }
}

fn drive(kinds: &[KnownTaskEventKind]) -> TaskView {
    let mut view = initial_view();
    for kind in kinds {
        advance(&mut view, kind);
    }
    view
}

fn call() -> CallId {
    CallId::generate(&SystemClock)
}

#[test]
fn t_task_state_9() {
    let c = call();
    let mut view = drive(&[
        KnownTaskEventKind::TaskCreated {
            goal: "g".to_owned(),
            opened_by: anybot_protocol::TaskOpener::You,
            bot: anybot_foundation::BotId::generate(&SystemClock),
            parent: None,
        },
        turn_started_kind(),
        KnownTaskEventKind::CardShown {
            action: "a".to_owned(),
        },
        KnownTaskEventKind::CardAnswered {
            answer: CardAnswer::Deny,
        },
        KnownTaskEventKind::TurnEnded {
            reason: anybot_protocol::TurnEndReason::Answered,
            reply_delivered: true,
        },
    ]);
    assert_eq!(view.status, TaskStatus::Unaccepted);
    assert_eq!(view.detail, Some(TaskDetail::StepRejected));
    advance(
        &mut view,
        &KnownTaskEventKind::Wake {
            reason: "user message".to_owned(),
        },
    );
    assert_eq!(view.status, TaskStatus::Running);
    let applied = [
        turn_started_kind(),
        KnownTaskEventKind::ToolIntent {
            call_id: c,
            name: "edit".to_owned(),
            args_summary: "file".to_owned(),
            level: ToolEffect::Irreversible,
        },
        KnownTaskEventKind::ToolStarted { call_id: c },
        KnownTaskEventKind::ToolCompleted {
            call_id: c,
            summary: "s".to_owned(),
            raw_id: None,
        },
        KnownTaskEventKind::ActionVerified {
            call_id: c,
            source: CheckSource::Program,
            evidence_ids: Vec::new(),
        },
        KnownTaskEventKind::TurnEnded {
            reason: anybot_protocol::TurnEndReason::Answered,
            reply_delivered: true,
        },
    ];
    for kind in &applied {
        advance(&mut view, kind);
    }
    assert_eq!(view.status, TaskStatus::Completed);
    assert_eq!(view.detail, Some(TaskDetail::Accepted));
}

#[test]
fn budget_raise_restores_running() {
    let mut view = initial_view();
    advance(
        &mut view,
        &KnownTaskEventKind::TaskCreated {
            goal: "g".to_owned(),
            opened_by: anybot_protocol::TaskOpener::You,
            bot: anybot_foundation::BotId::generate(&SystemClock),
            parent: None,
        },
    );
    advance(
        &mut view,
        &KnownTaskEventKind::BudgetLimit {
            item: BudgetItem::ToolCalls,
            limit: 10,
        },
    );
    for _ in 0..10 {
        let c = call();
        advance(
            &mut view,
            &KnownTaskEventKind::ToolIntent {
                call_id: c,
                name: "n".to_owned(),
                args_summary: "a".to_owned(),
                level: ToolEffect::ReadOnly,
            },
        );
        advance(&mut view, &KnownTaskEventKind::ToolStarted { call_id: c });
    }
    advance(
        &mut view,
        &KnownTaskEventKind::ToolIntent {
            call_id: call(),
            name: "n".to_owned(),
            args_summary: "a".to_owned(),
            level: ToolEffect::ReadOnly,
        },
    );
    assert_eq!(view.status, TaskStatus::Stuck);
    advance(
        &mut view,
        &KnownTaskEventKind::BudgetLimit {
            item: BudgetItem::ToolCalls,
            limit: 20,
        },
    );
    assert_eq!(view.status, TaskStatus::Running);
    advance(
        &mut view,
        &KnownTaskEventKind::TurnEnded {
            reason: anybot_protocol::TurnEndReason::Answered,
            reply_delivered: true,
        },
    );
    assert_eq!(view.status, TaskStatus::Completed);
}
