use proptest::prelude::*;
use proptest::strategy::ValueTree;
use proptest::test_runner::TestRunner;

use crate::fixtures::chat_record;
use crate::input::ChatRecord;
use crate::mask_accum_tests::{FIXED_LENS, Occurrences, one_case};

fn short_chat(prefix: &'static str, index: usize) -> BoxedStrategy<ChatRecord> {
    chat_record(prefix, index)
        .prop_map(|r| ChatRecord {
            text: r.text.chars().take(150).collect(),
            ..r
        })
        .boxed()
}

#[test]
fn t_context_mask_2() {
    let strategy = (
        prop::collection::vec(0usize..FIXED_LENS.len(), 16..22),
        prop::collection::vec(short_chat("u", 0), 1..3),
        prop::collection::vec(short_chat("b", 1), 0..3),
        4_000u64..20_000u64,
    );
    let mut runner = TestRunner::deterministic();
    let mut st = Occurrences::default();
    for _ in 0..1000 {
        let tree = strategy.new_tree(&mut runner).unwrap();
        let (idxs, users, bots, usable) = ValueTree::current(&tree);
        let lens: Vec<usize> = idxs
            .iter()
            .map(|i| *FIXED_LENS.get(i % FIXED_LENS.len()).unwrap_or(&300))
            .collect();
        one_case(&lens, users, bots, usable, &mut st).unwrap();
    }
    assert!(
        st.sent_cases * 2 >= st.cases,
        "most cases must build a request"
    );
    assert!(
        st.accumulate_collapses * 5 >= st.cases,
        "accumulated collapse must happen often"
    );
    assert!(
        st.margin_checks * 10 >= st.accumulate_collapses,
        "margin must be checked"
    );
    assert!(
        st.short_kept * 2 >= st.sent_cases,
        "short results must appear and stay unmasked"
    );
    assert!(
        st.masks_seen * 2 >= st.sent_cases,
        "masked form must appear"
    );
}
