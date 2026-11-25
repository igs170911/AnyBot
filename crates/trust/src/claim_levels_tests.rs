use proptest::prelude::*;

use crate::claim_levels::*;

fn link_strategy() -> BoxedStrategy<Link> {
    ((0usize..4usize), (0usize..2usize))
        .prop_map(|(kind, gate)| Link {
            kind: match kind {
                0 => EvidenceKind::ToolReturn,
                1 => EvidenceKind::UserWords,
                2 => EvidenceKind::BotHearsay,
                _ => EvidenceKind::ModelDerivation,
            },
            gate_checked: gate == 1,
        })
        .boxed()
}

fn scope_strategy() -> BoxedStrategy<Scope> {
    (0usize..3usize)
        .prop_map(|v| match v {
            0 => Scope::BotComputer,
            1 => Scope::UserComputer,
            _ => Scope::Phone,
        })
        .boxed()
}

fn claim_strategy() -> BoxedStrategy<Claim> {
    (
        prop::collection::vec(link_strategy(), 0..6),
        prop::option::of(scope_strategy()),
        prop::option::of(scope_strategy()),
    )
        .prop_map(|(links, verification, claim_scope)| Claim {
            text: "claim".to_owned(),
            links,
            verification: verification.map(|scope| Verification { scope }),
            claim_scope,
        })
        .boxed()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(2000))]

    #[test]
    fn t_claim_levels_1(claims in prop::collection::vec(claim_strategy(), 1..4)) {
        for claim in &claims {
            let level = claim_level(claim);
            prop_assert!(level <= ClaimLevel::Verified);
            if claim.verification.is_none() {
                prop_assert!(level < ClaimLevel::Verified);
            }
            if claim.links.is_empty() {
                prop_assert_eq!(level, ClaimLevel::Unknown);
            }
            let repeated = Claim {
                links: claim.links.repeat(2),
                ..claim.clone()
            };
            prop_assert_eq!(claim_level(&repeated), level);
        }
    }
}

#[test]
fn test_scopes_are_annotated_for_claim_levels_5() {
    let claim = Claim {
        text: "the box is offline".to_owned(),
        links: vec![Link {
            kind: EvidenceKind::ToolReturn,
            gate_checked: false,
        }],
        verification: Some(Verification {
            scope: Scope::BotComputer,
        }),
        claim_scope: Some(Scope::Phone),
    };
    assert_eq!(claim_level(&claim), ClaimLevel::SourceBacked);
    assert_eq!(
        unverified_scopes(&claim),
        vec![Scope::UserComputer, Scope::Phone]
    );
}
