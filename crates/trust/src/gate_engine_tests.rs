use proptest::prelude::*;

use crate::gate_engine::*;

fn bool_strategy() -> BoxedStrategy<bool> {
    (0usize..2usize).prop_map(|v| v == 1).boxed()
}

fn call_strategy(layer: GateLayer) -> BoxedStrategy<LayerCall> {
    (bool_strategy(), bool_strategy(), bool_strategy())
        .prop_map(move |(deny, ask, allow)| LayerCall {
            layer,
            deny,
            ask,
            allow,
        })
        .boxed()
}

fn calls_strategy() -> BoxedStrategy<Vec<LayerCall>> {
    (
        call_strategy(GateLayer::Floor),
        call_strategy(GateLayer::Admin),
        call_strategy(GateLayer::User),
        call_strategy(GateLayer::Default),
    )
        .prop_map(|(f, a, u, d)| vec![f, a, u, d])
        .boxed()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(5000))]

    #[test]
    fn t_safety_gates_1(calls in calls_strategy()) {
        let decision = precedence_decision(&calls);
        let expected = LAYER_ORDER
            .iter()
            .find_map(|layer| {
                let call = calls.iter().find(|c| c.layer == *layer)?;
                Outcome::from_opinion(call.opinion()).map(|outcome| (*layer, outcome))
            });
        prop_assert_eq!(decision, expected);
        for call in &calls {
            if call.deny && call.allow {
                prop_assert_eq!(call.opinion(), Opinion::Deny);
            }
        }
    }
}

#[test]
fn test_precedence_skips_layers_without_opinion() {
    let calls = vec![
        LayerCall {
            layer: GateLayer::Floor,
            deny: false,
            ask: false,
            allow: false,
        },
        LayerCall {
            layer: GateLayer::Admin,
            deny: false,
            ask: true,
            allow: false,
        },
        LayerCall {
            layer: GateLayer::User,
            deny: false,
            ask: false,
            allow: true,
        },
    ];
    assert_eq!(
        precedence_decision(&calls),
        Some((GateLayer::Admin, Outcome::Ask))
    );
}
