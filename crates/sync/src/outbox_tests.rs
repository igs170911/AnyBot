use proptest::prelude::*;

use crate::outbox::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    #[test]
    fn t_sync_outbox_4(attempts in prop::collection::vec(0u64..60, 200)) {
        for attempt in &attempts {
            let delay = retry_delay_secs(*attempt);
            prop_assert!(delay >= 1);
            prop_assert!(delay <= RETRY_CAP_SECONDS);
            if *attempt < 5 {
                prop_assert_eq!(delay, 1u64 << *attempt);
            }
        }
        prop_assert_eq!(retry_schedule(7), vec![1, 2, 4, 8, 16, 30, 30]);
    }
}
