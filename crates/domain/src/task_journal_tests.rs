use proptest::prelude::*;

use super::{JournalOp, TaskJournal};

fn op_strategy() -> BoxedStrategy<JournalOp> {
    prop_oneof![
        Just(JournalOp::OpenTask),
        Just(JournalOp::StateChange),
        Just(JournalOp::EvidenceAdd),
        Just(JournalOp::ClearChat),
        Just(JournalOp::NewConversation),
    ]
    .boxed()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(200))]

    #[test]
    fn t_task_object_3_journal_invariants(ops in prop::collection::vec(op_strategy(), 1..60)) {
        let mut journal = TaskJournal::new();
        let mut opens = 0usize;
        let mut prev_counts: Vec<usize> = Vec::new();
        for op in &ops {
            journal.apply(op);
            if *op == JournalOp::OpenTask {
                opens = opens.saturating_add(1);
            }
            prop_assert!(journal.shape_matches());
            prop_assert_eq!(journal.open_task_count(), opens);
            for (slot, count) in prev_counts.iter_mut().zip(0..opens) {
                let now = journal.record_count(count).unwrap_or(0);
                prop_assert!(now >= *slot);
                slot.clone_from(&now);
            }
            for i in prev_counts.len()..opens {
                prev_counts.push(journal.record_count(i).unwrap_or(0));
            }
        }
        let before = journal.clone();
        journal.apply(&JournalOp::ClearChat);
        journal.apply(&JournalOp::NewConversation);
        prop_assert!(journal.journal_intact_after_clear(&before));
    }
}

#[test]
fn test_journal_contiguous_seq_record_model() {
    let mut journal = TaskJournal::new();
    journal.apply(&JournalOp::OpenTask);
    journal.apply(&JournalOp::StateChange);
    journal.apply(&JournalOp::EvidenceAdd);
    assert_eq!(journal.record_count(0), Some(2));
    journal.apply(&JournalOp::ClearChat);
    assert_eq!(journal.record_count(0), Some(2));
}
