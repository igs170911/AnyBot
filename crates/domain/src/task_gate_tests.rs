use anybot_foundation::CallId;
use anybot_foundation::SystemClock;
use anybot_protocol::{
    BudgetItem, CheckSource, Criterion, DoneWhenChange, KnownTaskEventKind, Proposer, TaskDetail,
    TaskStatus, ToolEffect,
};

use crate::task_engine::advance;
use crate::task_view::{TaskView, initial_view};

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
fn t_task_done_gate_2_reply_only_completes_as_replied() {
    let view = drive(&[
        KnownTaskEventKind::TaskCreated {
            goal: "g".to_owned(),
        },
        KnownTaskEventKind::TurnStarted,
        KnownTaskEventKind::TurnEnded {
            reply_delivered: true,
        },
    ]);
    assert_eq!(view.status, TaskStatus::Completed);
    assert_eq!(view.detail, Some(TaskDetail::Replied));
}

#[test]
fn t_task_done_gate_3_auto_checks_all_pass_completes_as_accepted() {
    let c = call();
    let view = drive(&[
        KnownTaskEventKind::TaskCreated {
            goal: "g".to_owned(),
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
            reply_delivered: true,
        },
    ]);
    assert_eq!(view.status, TaskStatus::Completed);
    assert_eq!(view.detail, Some(TaskDetail::Accepted));
    assert!(view.criteria_all_passed());
}

#[test]
fn t_task_done_gate_4_acceptance_unavailable_never_completes() {
    let view = drive(&[
        KnownTaskEventKind::TaskCreated {
            goal: "g".to_owned(),
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
            reply_delivered: true,
        },
    ]);
    assert_eq!(view.status, TaskStatus::Unaccepted);
    assert_eq!(view.detail, Some(TaskDetail::NoVisibleResult));
}

#[test]
fn t_task_done_gate_5_bot_loosening_waits_for_user_decision() {
    let set = DoneWhenChange::SetItems {
        items: vec![
            Criterion::FileExists {
                path: "a".to_owned(),
            },
            Criterion::FileExists {
                path: "b".to_owned(),
            },
        ],
    };
    let mut view = drive(&[
        KnownTaskEventKind::TaskCreated {
            goal: "g".to_owned(),
        },
        KnownTaskEventKind::DoneWhenChange {
            by: Proposer::User,
            change: set,
            before: vec![],
            after: vec![],
            reason: "set".to_owned(),
        },
        KnownTaskEventKind::DoneWhenChange {
            by: Proposer::Bot,
            change: DoneWhenChange::RemoveItem { index: 1 },
            before: vec![
                Criterion::FileExists {
                    path: "a".to_owned(),
                },
                Criterion::FileExists {
                    path: "b".to_owned(),
                },
            ],
            after: vec![Criterion::FileExists {
                path: "a".to_owned(),
            }],
            reason: "too strict".to_owned(),
        },
        KnownTaskEventKind::TurnEnded {
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
fn t_task_done_gate_6_raised_limit_makes_budget_pass_again() {
    let mut view = initial_view();
    advance(
        &mut view,
        &KnownTaskEventKind::TaskCreated {
            goal: "g".to_owned(),
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
    assert_eq!(view.status, TaskStatus::Stuck);
    advance(
        &mut view,
        &KnownTaskEventKind::BudgetLimit {
            item: BudgetItem::ToolCalls,
            limit: 12,
        },
    );
    assert_eq!(view.status, TaskStatus::Running);
    advance(
        &mut view,
        &KnownTaskEventKind::TurnEnded {
            reply_delivered: true,
        },
    );
    assert_eq!(view.status, TaskStatus::Completed);
}
