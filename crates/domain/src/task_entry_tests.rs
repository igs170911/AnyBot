use proptest::prelude::*;

use super::{EntryInput, EntryTracker};

fn input_strategy() -> BoxedStrategy<EntryInput> {
    prop_oneof![
        Just(EntryInput::UserMessage),
        Just(EntryInput::SteerMessage),
        Just(EntryInput::RoutineFire),
        Just(EntryInput::GoalSet),
        Just(EntryInput::GoalNextRound),
        Just(EntryInput::FollowUpWake),
    ]
    .boxed()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]

    #[test]
    fn t_task_entry_4_entries_and_turns_are_consistent(
        head in prop::collection::vec(Just(EntryInput::UserMessage), 1..3),
        tail in prop::collection::vec(input_strategy(), 0..60),
    ) {
        let mut tracker = EntryTracker::new();
        for input in head.iter().chain(tail.iter()) {
            tracker.handle(*input);
            prop_assert!(tracker.task_count_matches_entries());
            prop_assert!(tracker.every_turn_in_one_task());
            prop_assert!(tracker.wakes_reuse_tasks());
        }
    }
}

#[test]
fn test_entry_steer_joins_current_task() {
    let mut tracker = EntryTracker::new();
    tracker.handle(EntryInput::UserMessage);
    tracker.handle(EntryInput::SteerMessage);
    tracker.handle(EntryInput::FollowUpWake);
    assert_eq!(tracker.tasks, 1);
    assert_eq!(tracker.turns_per_task, vec![3]);
}
