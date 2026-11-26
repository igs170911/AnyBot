use proptest::prelude::*;

use crate::gate_engine::*;
use crate::gate_full::*;
use crate::trust_floor::*;

fn any_floor_action() -> BoxedStrategy<FloorAction> {
    prop_oneof![
        Just(FloorAction::Command {
            line: "sudo rm -rf /nowhere".to_owned(),
            bot_machine: true,
            has_sandbox: true,
        }),
        Just(FloorAction::FilePath {
            path: "~/.ssh/id_ed25519".to_owned(),
        }),
        Just(FloorAction::ClickTarget {
            app: "AnyBot".to_owned(),
            label: "allow".to_owned(),
        }),
        Just(FloorAction::Url {
            host: "169.254.169.254".to_owned(),
            port: 80,
        }),
    ]
    .boxed()
}

fn engine_input(action: FloorAction) -> BoxedStrategy<EngineInput> {
    (
        (0usize..2usize),
        (0usize..2usize),
        (0usize..2usize),
        (0usize..2usize),
        (0usize..2usize),
    )
        .prop_map(
            move |(yolo, direct, mdm, ask_time, six_class)| EngineInput {
                action: action.clone(),
                modes: ModeFlags {
                    six_class: six_class == 1,
                    ask_every_time: ask_time == 1,
                    yolo: yolo == 1,
                    direct_mode: direct == 1,
                    mdm_locks_direct: mdm == 1,
                },
                styles: StyleFlags {
                    ask_style_hit: false,
                    allow_style_hit: true,
                    always_hit: true,
                    session_allowed: true,
                },
                admin_hook: Opinion::Allow,
                user_hook: Opinion::Allow,
                default_rule: Opinion::Allow,
            },
        )
        .boxed()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(2000))]

    #[test]
    fn t_trust_floor_4(input in any_floor_action().prop_flat_map(engine_input)) {
        let decision = decide(&input);
        prop_assert_eq!(decision.outcome, Outcome::Deny);
        prop_assert_eq!(decision.layer, GateLayer::Floor);
    }
}
