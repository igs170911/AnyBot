use proptest::prelude::*;

use anybot_protocol::{BudgetItem, KnownTaskEventKind, TaskDetail, TaskStatus};

use crate::task_budget::fixtures::{asks, call, created, drive, intent, pair, turns_ended};

fn turn_started_kind() -> KnownTaskEventKind {
    KnownTaskEventKind::TurnStarted {
        turn: 1,
        initiator: anybot_protocol::TurnInitiator::You {
            device: anybot_foundation::DeviceId::generate(&anybot_foundation::SystemClock),
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

#[derive(Debug, Clone)]
struct BudgetCase {
    events: Vec<KnownTaskEventKind>,
    expect: Option<(BudgetItem, u64, u64)>,
}

fn build_case(item: BudgetItem, limit: u64, extra: u64) -> BudgetCase {
    let mut head = vec![
        created(),
        KnownTaskEventKind::BudgetLimit { item, limit },
        turn_started_kind(),
    ];
    let tail = vec![turns_ended()];
    let expect;
    if item == BudgetItem::Steps {
        head.extend(asks(usize::try_from(limit).unwrap_or(0)));
        if extra == 1 {
            head.push(KnownTaskEventKind::BudgetUsage {
                item: BudgetItem::Steps,
                delta: 1,
            });
            expect = Some((item, limit, limit));
        } else {
            expect = None;
        }
    } else if item == BudgetItem::ToolCalls {
        head.extend(pair(usize::try_from(limit).unwrap_or(0)));
        if extra == 1 {
            head.push(intent());
            head.push(KnownTaskEventKind::ToolStarted { call_id: call() });
            expect = Some((item, limit, limit));
        } else {
            expect = None;
        }
    } else {
        head.extend((0..limit).map(|_| KnownTaskEventKind::BudgetUsage { item, delta: 1 }));
        if extra == 1 {
            head.push(KnownTaskEventKind::BudgetUsage { item, delta: 1 });
            expect = Some((item, limit.saturating_add(1), limit));
        } else {
            expect = None;
        }
    }
    head.extend(tail);
    BudgetCase {
        events: head,
        expect,
    }
}

fn budget_cases() -> BoxedStrategy<BudgetCase> {
    (
        prop_oneof![
            Just(BudgetItem::Steps),
            Just(BudgetItem::ToolCalls),
            Just(BudgetItem::Tokens),
        ],
        1u64..40u64,
        0u64..2u64,
    )
        .prop_map(|(item, limit, extra)| build_case(item, limit, extra))
        .boxed()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    #[test]
    fn t_task_budget_1(case in budget_cases()) {
        let view = drive(&case.events);
        if let Some((item, used, limit)) = case.expect {
            prop_assert_eq!(view.status, TaskStatus::Stuck);
            prop_assert_eq!(
                view.detail,
                Some(TaskDetail::OverBudget { item, used, limit })
            );
            prop_assert_eq!(view.budget.used(item), used);
        } else {
            prop_assert_ne!(view.status, TaskStatus::Stuck);
        }
    }
}
