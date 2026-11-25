use std::future::{Future, ready};
use std::pin::Pin;
use std::time::Duration;

use anybot_foundation::{Clock, DeviceId, RecordId, TimestampMs};
use proptest::prelude::*;

struct Skewed {
    base: u64,
    skew: i64,
}

impl Clock for Skewed {
    fn now_ms(&self) -> TimestampMs {
        let value = i64::try_from(self.base).unwrap_or(i64::MAX);
        TimestampMs::from_millis(value.saturating_add(self.skew).max(0).cast_unsigned())
    }

    fn sleep(&self, _span: Duration) -> Pin<Box<dyn Future<Output = ()> + Send>> {
        Box::pin(ready(()))
    }

    fn sleep_until(&self, _deadline: TimestampMs) -> Pin<Box<dyn Future<Output = ()> + Send>> {
        Box::pin(ready(()))
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(10))]

    #[test]
    fn t_sync_id_1(seeds in prop::collection::vec(0i64..3_600_000, 10), base in 1_700_000_000_000u64..1_700_000_060_000) {
        let mut all: Vec<String> = vec![];
        for seed in seeds {
            let clock = Skewed { base, skew: seed };
            for _ in 0..100_000 {
                all.push(RecordId::generate(&clock).as_str().clone());
            }
        }
        let unique: std::collections::BTreeSet<String> = all.iter().cloned().collect();
        prop_assert_eq!(unique.len(), all.len());
    }

    #[test]
    fn test_device_ids_are_unique(seeds in prop::collection::vec(0i64..3_600_000, 10), base in 1_700_000_000_000u64..1_700_000_060_000) {
        let mut all: Vec<String> = vec![];
        for seed in seeds {
            let clock = Skewed { base, skew: seed };
            for _ in 0..10_000 {
                all.push(DeviceId::generate(&clock).as_str().clone());
            }
        }
        let unique: std::collections::BTreeSet<String> = all.iter().cloned().collect();
        prop_assert_eq!(unique.len(), all.len());
    }
}
