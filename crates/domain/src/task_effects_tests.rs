use proptest::prelude::*;

use anybot_protocol::{ReconcileOutcome, ToolEffect};

use super::{ResumeAction, resume_action};

fn level_strategy() -> BoxedStrategy<ToolEffect> {
    prop_oneof![
        Just(ToolEffect::ReadOnly),
        Just(ToolEffect::Repeatable),
        Just(ToolEffect::Irreversible),
    ]
    .boxed()
}

fn reconcile_strategy() -> BoxedStrategy<Option<ReconcileOutcome>> {
    prop_oneof![
        Just(None),
        Just(Some(ReconcileOutcome::Done)),
        Just(Some(ReconcileOutcome::NotDone)),
        Just(Some(ReconcileOutcome::Unknown)),
    ]
    .boxed()
}

#[test]
fn t_task_effects_1_three_levels_give_three_plans() {
    assert_eq!(
        resume_action(ToolEffect::ReadOnly, None),
        ResumeAction::Redo
    );
    assert_eq!(
        resume_action(ToolEffect::Repeatable, None),
        ResumeAction::Reconcile
    );
    assert_eq!(
        resume_action(ToolEffect::Irreversible, None),
        ResumeAction::WaitUser
    );
}

#[test]
fn t_task_effects_2_missing_level_defaults_to_irreversible() {
    assert_eq!(
        anybot_protocol::effect_level(None),
        ToolEffect::Irreversible
    );
    assert_eq!(
        resume_action(anybot_protocol::effect_level(None), None),
        ResumeAction::WaitUser
    );
}

#[test]
fn t_task_effects_3_failed_reconcile_waits_for_user() {
    assert_eq!(
        resume_action(ToolEffect::Repeatable, Some(ReconcileOutcome::Unknown),),
        ResumeAction::WaitUser
    );
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(2000))]

    #[test]
    fn t_task_effects_4_irreversible_never_redoes(
        level in level_strategy(),
        reconcile in reconcile_strategy(),
    ) {
        let plan = resume_action(level, reconcile);
        if level == ToolEffect::Irreversible {
            prop_assert_eq!(plan, ResumeAction::WaitUser);
        }
        prop_assert!(plan != ResumeAction::WaitUser || level != ToolEffect::ReadOnly);
    }
}
