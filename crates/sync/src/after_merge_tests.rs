use proptest::prelude::*;

use crate::after_merge::*;

fn server_strategy() -> BoxedStrategy<Vec<StreamItem>> {
    prop::collection::vec(1u64..40, 20..35)
        .prop_map(|seqs| {
            let uniq: std::collections::BTreeSet<u64> = seqs.into_iter().collect();
            uniq.iter()
                .map(|s| StreamItem {
                    seq: *s,
                    body: format!("item {s}"),
                })
                .collect()
        })
        .boxed()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(300))]

    #[test]
    fn replica_model_ignores_duplicates_and_order(
        server in server_strategy(),
        start in 0u64..4u64,
        pushes in prop::collection::vec(0usize..8, 3),
    ) {
        let mut replica = Replica::default();
        for push in &pushes {
            let visible: Vec<StreamItem> = server
                .iter()
                .filter(|i| i.seq > start)
                .cloned()
                .collect();
            let mut push_items = visible.clone();
            if *push % 2 == 0 {
                push_items.push(visible.first().cloned().unwrap_or(StreamItem { seq: 0, body: "extra".to_owned() }));
                let last = push_items.len().saturating_sub(1);
                push_items.swap(0, last);
            }
            replica.apply(&StreamDelta { from_seq: start, items: push_items });
        }
        let expected: Vec<StreamItem> = server
            .iter()
            .filter(|i| i.seq > start)
            .cloned()
            .collect();
        prop_assert_eq!(&replica.items, &expected);
    }
}

#[test]
fn test_replica_tracks_gaps() {
    let mut replica = Replica::default();
    replica.apply(&StreamDelta {
        from_seq: 0,
        items: vec![
            StreamItem {
                seq: 1,
                body: "a".to_owned(),
            },
            StreamItem {
                seq: 3,
                body: "c".to_owned(),
            },
        ],
    });
    assert_eq!(replica.gaps, vec![2]);
    assert_eq!(replica.resync_from(), Some(1));
    assert_eq!(
        delta_from(
            &[StreamItem {
                seq: 4,
                body: "d".to_owned()
            }],
            replica.through,
        )
        .items
        .len(),
        1
    );
}
