use anybot_foundation::CallId;
use anybot_foundation::SystemClock;
use anybot_protocol::{
    CheckSource, Criterion, DoneWhenChange, KnownTaskEventKind, Proposer, TaskDetail, TaskStatus,
    ToolEffect,
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
fn t_task_done_gate_2() {
    let view = drive(&[
        KnownTaskEventKind::TaskCreated {
            goal: "g".to_owned(),
            opened_by: anybot_protocol::TaskOpener::You,
            bot: anybot_foundation::BotId::generate(&SystemClock),
            parent: None,
        },
        turn_started_kind(),
        KnownTaskEventKind::TurnEnded {
            reason: anybot_protocol::TurnEndReason::Answered,
            reply_delivered: true,
        },
    ]);
    assert_eq!(view.status, TaskStatus::Completed);
    assert_eq!(view.detail, Some(TaskDetail::Replied));
}

#[test]
fn t_task_done_gate_3() {
    let c = call();
    let view = drive(&[
        KnownTaskEventKind::TaskCreated {
            goal: "g".to_owned(),
            opened_by: anybot_protocol::TaskOpener::You,
            bot: anybot_foundation::BotId::generate(&SystemClock),
            parent: None,
        },
        KnownTaskEventKind::DoneWhenChange {
            by: Proposer::User,
            change: DoneWhenChange::SetItems {
                items: vec![
                    Criterion::FileExists {
                        path: "out.txt".to_owned(),
                    },
                    Criterion::DeliveredToUser {
                        file: "out.txt".to_owned(),
                    },
                ],
            },
            before: vec![],
            after: vec![],
            reason: "set".to_owned(),
        },
        KnownTaskEventKind::ToolIntent {
            call_id: c,
            name: "write".to_owned(),
            args_summary: "a".to_owned(),
            level: ToolEffect::Irreversible,
        },
        KnownTaskEventKind::ToolStarted { call_id: c },
        KnownTaskEventKind::ActionVerified {
            call_id: c,
            source: CheckSource::Program,
            evidence_ids: Vec::new(),
        },
        KnownTaskEventKind::CriterionPassed {
            index: 0,
            source: CheckSource::Program,
        },
        KnownTaskEventKind::CriterionPassed {
            index: 1,
            source: CheckSource::Program,
        },
        KnownTaskEventKind::TurnEnded {
            reason: anybot_protocol::TurnEndReason::Answered,
            reply_delivered: true,
        },
    ]);
    assert_eq!(view.status, TaskStatus::Completed);
    assert_eq!(view.detail, Some(TaskDetail::Accepted));
    assert!(view.criteria_all_passed());
}

#[test]
fn t_task_done_gate_4() {
    let view = drive(&[
        KnownTaskEventKind::TaskCreated {
            goal: "g".to_owned(),
            opened_by: anybot_protocol::TaskOpener::You,
            bot: anybot_foundation::BotId::generate(&SystemClock),
            parent: None,
        },
        KnownTaskEventKind::DoneWhenChange {
            by: Proposer::User,
            change: DoneWhenChange::SetItems {
                items: vec![Criterion::ModelJudgement {
                    text: "good".to_owned(),
                }],
            },
            before: vec![],
            after: vec![],
            reason: "set".to_owned(),
        },
        KnownTaskEventKind::AcceptanceUnavailable,
        KnownTaskEventKind::TurnEnded {
            reason: anybot_protocol::TurnEndReason::Answered,
            reply_delivered: true,
        },
    ]);
    assert_eq!(view.status, TaskStatus::Unaccepted);
    assert_eq!(view.detail, Some(TaskDetail::NoVisibleResult));
}

fn change_event(
    by: Proposer,
    change: DoneWhenChange,
    before: Vec<Criterion>,
    after: Vec<Criterion>,
    reason: &str,
) -> KnownTaskEventKind {
    KnownTaskEventKind::DoneWhenChange {
        by,
        change,
        before,
        after,
        reason: reason.to_owned(),
    }
}

fn exists(path: &str) -> Criterion {
    Criterion::FileExists {
        path: path.to_owned(),
    }
}

#[test]
fn t_task_done_gate_5() {
    let set = DoneWhenChange::SetItems {
        items: vec![exists("a"), exists("b")],
    };
    let mut view = drive(&[
        KnownTaskEventKind::TaskCreated {
            goal: "g".to_owned(),
            opened_by: anybot_protocol::TaskOpener::You,
            bot: anybot_foundation::BotId::generate(&SystemClock),
            parent: None,
        },
        change_event(Proposer::User, set.clone(), Vec::new(), Vec::new(), "set"),
        change_event(
            Proposer::Bot,
            DoneWhenChange::RemoveItem { index: 1 },
            vec![exists("a"), exists("b")],
            vec![exists("a")],
            "too strict",
        ),
        KnownTaskEventKind::TurnEnded {
            reason: anybot_protocol::TurnEndReason::Answered,
            reply_delivered: true,
        },
    ]);
    assert_eq!(view.criteria.len(), 2);
    assert_ne!(view.status, TaskStatus::AwaitingUser);
    assert_eq!(view.pending_proposals.len(), 1);
    let proposal = view.pending_proposals.first().unwrap();
    assert_eq!(proposal.by, Proposer::Bot);
    assert_eq!(proposal.before.len(), 2);
    assert_eq!(proposal.after.len(), 1);
    advance(
        &mut view,
        &KnownTaskEventKind::DoneWhenAppliedByUser {
            change: DoneWhenChange::RemoveItem { index: 1 },
        },
    );
    assert_eq!(view.criteria.len(), 1);
    assert!(view.pending_proposals.is_empty());
}

#[test]
fn t_task_done_gate_6() {
    let set = DoneWhenChange::SetItems {
        items: vec![exists("a"), exists("b")],
    };
    let view = drive(&[
        KnownTaskEventKind::TaskCreated {
            goal: "g".to_owned(),
            opened_by: anybot_protocol::TaskOpener::You,
            bot: anybot_foundation::BotId::generate(&SystemClock),
            parent: None,
        },
        change_event(Proposer::User, set.clone(), Vec::new(), Vec::new(), "set"),
        KnownTaskEventKind::DoneWhenChange {
            by: Proposer::Bot,
            change: DoneWhenChange::AddItem {
                item: Criterion::CommandExitZero {
                    command: "make test".to_owned(),
                },
            },
            before: vec![],
            after: vec![],
            reason: "more checks".to_owned(),
        },
    ]);
    assert_eq!(view.criteria.len(), 3);
    assert!(view.pending_proposals.is_empty());
}
