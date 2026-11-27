use crate::mailbox_model::{DELIVERIES, Op, SHORT_PASS_SECS, plan};

#[test]
fn the_same_seed_gives_the_same_plan() -> Result<(), String> {
    let first = plan(7)?;
    let second = plan(7)?;
    assert_eq!(first.ops, second.ops);
    assert_eq!(first.executors, second.executors);
    assert_eq!(first.moves, second.moves);
    Ok(())
}

#[test]
fn a_plan_hands_out_every_letter_and_ends_with_a_pass() -> Result<(), String> {
    let built = plan(11)?;
    assert_eq!(built.executors.len(), DELIVERIES);
    assert!(matches!(
        built.ops.last(),
        Some(Op::Pass {
            secs: SHORT_PASS_SECS
        })
    ));
    assert!(built.moves > 0);
    Ok(())
}

#[test]
fn a_plan_has_one_deliver_per_slot_and_many_passes() -> Result<(), String> {
    let built = plan(13)?;
    let delivered = built
        .ops
        .iter()
        .filter(|op| matches!(op, Op::Deliver { .. }))
        .count();
    assert_eq!(delivered, DELIVERIES);
    let passes = built
        .ops
        .iter()
        .filter(|op| matches!(op, Op::Pass { .. }))
        .count();
    assert!(passes > 10);
    Ok(())
}
