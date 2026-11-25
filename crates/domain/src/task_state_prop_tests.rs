use proptest::prelude::*;

use anybot_foundation::{CallId, SystemClock};
use anybot_protocol::Criterion;
use anybot_protocol::DoneWhenChange;
use anybot_protocol::Proposer;
use anybot_protocol::{
    BudgetItem, CheckSource, DeviceReportKind, KnownTaskEventKind, TaskDetail, TaskStatus,
    ToolEffect, detail_matches_status,
};

use crate::task_engine::advance;
use crate::task_view::{TaskView, initial_view};

fn criterion_events() -> BoxedStrategy<KnownTaskEventKind> {
    prop_oneof![
        (0usize..4usize, 0usize..2usize).prop_map(|(index, p)| {
            if p == 1 {
                KnownTaskEventKind::CriterionPassed {
                    index,
                    source: CheckSource::Program,
                }
            } else {
                KnownTaskEventKind::CriterionFailed {
                    index,
                    source: CheckSource::Program,
                    observed: "o".to_owned(),
                }
            }
        }),
        (0usize..2usize, 0usize..2usize).prop_map(|(c, v)| KnownTaskEventKind::ClaimChecked {
            covers_claim: c == 1,
            verified: v == 1,
        }),
    ]
    .boxed()
}

fn tool_events() -> BoxedStrategy<KnownTaskEventKind> {
    let call = Just(CallId::generate(&SystemClock));
    prop_oneof![
        call.prop_map(|c| KnownTaskEventKind::ToolStarted { call_id: c }),
        call.prop_map(|c| KnownTaskEventKind::OutcomeUnknown { call_id: c }),
        call.prop_map(|c| KnownTaskEventKind::ActionVerified {
            call_id: c,
            source: CheckSource::Program,
        }),
        call.prop_map(|c| KnownTaskEventKind::ToolIntent {
            call_id: c,
            name: "n".to_owned(),
            args_summary: "a".to_owned(),
            level: ToolEffect::Irreversible,
        }),
    ]
    .boxed()
}

fn budget_events() -> BoxedStrategy<KnownTaskEventKind> {
    prop_oneof![
        (0u64..1200u64).prop_map(|used| KnownTaskEventKind::BudgetUsage {
            item: BudgetItem::Tokens,
            delta: used,
        }),
        Just(KnownTaskEventKind::BudgetLimit {
            item: BudgetItem::Tokens,
            limit: 1000,
        }),
    ]
    .boxed()
}

fn report_events() -> BoxedStrategy<KnownTaskEventKind> {
    prop_oneof![
        Just(DeviceReportKind::ThumbsUp).prop_map(|kind| KnownTaskEventKind::DeviceReport {
            kind,
            file: "f".to_owned(),
        }),
        Just(DeviceReportKind::Shown).prop_map(|kind| KnownTaskEventKind::DeviceReport {
            kind,
            file: "f".to_owned(),
        }),
        Just(DeviceReportKind::ThumbsDown).prop_map(|kind| KnownTaskEventKind::DeviceReport {
            kind,
            file: "f".to_owned(),
        }),
        Just(KnownTaskEventKind::TurnStarted),
        Just(KnownTaskEventKind::TurnEnded {
            reply_delivered: true,
        }),
        Just("w".to_owned()).prop_map(|reason| KnownTaskEventKind::Wake { reason }),
        Just(KnownTaskEventKind::AcceptanceUnavailable),
    ]
    .boxed()
}

fn user_call_seq() -> BoxedStrategy<KnownTaskEventKind> {
    prop_oneof![
        criterion_events(),
        tool_events(),
        budget_events(),
        report_events()
    ]
    .boxed()
}

fn criterion_kinds(results: &[bool]) -> Vec<KnownTaskEventKind> {
    results
        .iter()
        .enumerate()
        .map(|(index, pass)| {
            if *pass {
                KnownTaskEventKind::CriterionPassed {
                    index,
                    source: CheckSource::Program,
                }
            } else {
                KnownTaskEventKind::CriterionFailed {
                    index,
                    source: CheckSource::Program,
                    observed: "o".to_owned(),
                }
            }
        })
        .collect()
}

fn gate_events(claims_ok: usize, unknowns: usize, over: usize) -> Vec<KnownTaskEventKind> {
    let mut out = vec![];
    for _ in 0..claims_ok {
        out.push(KnownTaskEventKind::ClaimChecked {
            covers_claim: false,
            verified: true,
        });
    }
    for _ in 0..unknowns {
        out.push(KnownTaskEventKind::OutcomeUnknown {
            call_id: CallId::generate(&SystemClock),
        });
    }
    out.push(KnownTaskEventKind::BudgetLimit {
        item: BudgetItem::Tokens,
        limit: 1000,
    });
    out.push(KnownTaskEventKind::BudgetUsage {
        item: BudgetItem::Tokens,
        delta: u64::try_from(over).unwrap_or(0),
    });
    out.push(KnownTaskEventKind::TurnEnded {
        reply_delivered: true,
    });
    out
}

fn run_kinds(view: &mut TaskView, kinds: &[KnownTaskEventKind]) {
    for kind in kinds {
        advance(view, kind);
    }
}

fn head_kinds(n_criteria: usize) -> Vec<KnownTaskEventKind> {
    let items: Vec<Criterion> = (0..n_criteria)
        .map(|i| Criterion::FileExists {
            path: format!("f{i}"),
        })
        .collect();
    vec![
        KnownTaskEventKind::TaskCreated {
            goal: "g".to_owned(),
        },
        KnownTaskEventKind::DoneWhenChange {
            by: Proposer::User,
            change: DoneWhenChange::SetItems { items },
            before: vec![],
            after: vec![],
            reason: "set".to_owned(),
        },
    ]
}

fn state_invariants(view: &TaskView, thumbs_seen: bool) -> bool {
    let Some(detail) = &view.detail else {
        return false;
    };
    if !detail_matches_status(view.status, detail) {
        return false;
    }
    if view.status == TaskStatus::Completed && !view.completion_ready() {
        return false;
    }
    if matches!(detail, TaskDetail::Delivered) && !view.evidence.delivered_reported {
        return false;
    }
    if matches!(detail, TaskDetail::UserConfirmed) && !view.report.user_confirmed {
        return false;
    }
    if view.report.user_confirmed != thumbs_seen {
        return false;
    }
    true
}

fn detail_rank(detail: &TaskDetail) -> u8 {
    if *detail == TaskDetail::UserConfirmed {
        return 3;
    }
    if *detail == TaskDetail::Delivered {
        return 2;
    }
    if *detail == TaskDetail::Accepted {
        return 1;
    }
    0
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    #[test]
    fn t_task_state_7_invariants_hold_at_every_prefix(
        tail in prop::collection::vec(user_call_seq(), 1..25),
        n_criteria in 0usize..4usize,
    ) {
        let mut view = initial_view();
        for kind in head_kinds(n_criteria) {
            advance(&mut view, &kind);
        }
        let mut thumbs_seen = false;
        let mut best_rank = 0u8;
        for kind in tail {
            if matches!(kind, KnownTaskEventKind::DeviceReport {
                kind: DeviceReportKind::ThumbsUp | DeviceReportKind::Opened, ..
            }) {
                thumbs_seen = true;
            }
            advance(&mut view, &kind);
            prop_assert!(state_invariants(&view, thumbs_seen));
            if view.status == TaskStatus::Completed {
                let rank = detail_rank(view.detail.as_ref().unwrap());
                prop_assert!(rank >= best_rank);
                best_rank = rank.max(best_rank);
            }
        }
    }

    #[test]
    fn t_task_done_gate_1_completed_iff_four_conditions(
        results in prop::collection::vec(proptest::bool::ANY, 1..4),
        claims_ok in 0usize..2usize,
        unknowns in 0usize..2usize,
        over in 0usize..1200usize,
        thumbs in proptest::bool::ANY,
    ) {
        let mut view = initial_view();
        run_kinds(&mut view, &head_kinds(results.len()));
        run_kinds(&mut view, &criterion_kinds(&results));
        run_kinds(&mut view, &gate_events(claims_ok, unknowns, over));
        if thumbs {
            advance(
                &mut view,
                &KnownTaskEventKind::DeviceReport {
                    kind: DeviceReportKind::ThumbsUp,
                    file: "f".to_owned(),
                },
            );
        }
        let all_pass = results.iter().all(|p| *p);
        let ready = all_pass && claims_ok == 0 && unknowns == 0 && over < 1000;
        prop_assert_eq!(view.status == TaskStatus::Completed, ready);
    }
}
