use std::future::{Future, ready};
use std::pin::Pin;
use std::time::Duration;

use anybot_foundation::{Clock, TimestampMs};
use proptest::prelude::*;

use crate::always_list::*;

struct FixedClock(u64);

impl Clock for FixedClock {
    fn now_ms(&self) -> TimestampMs {
        TimestampMs::from_millis(self.0)
    }

    fn sleep(&self, _span: Duration) -> Pin<Box<dyn Future<Output = ()> + Send>> {
        Box::pin(ready(()))
    }

    fn sleep_until(&self, _deadline: TimestampMs) -> Pin<Box<dyn Future<Output = ()> + Send>> {
        Box::pin(ready(()))
    }
}

fn source_strategy() -> BoxedStrategy<AddSource> {
    (0usize..4usize)
        .prop_map(|v| match v {
            0 => AddSource::UserCardPress,
            1 => AddSource::WebContent,
            2 => AddSource::ChannelMessage,
            _ => AddSource::BotReply,
        })
        .boxed()
}

fn segment_strategy() -> BoxedStrategy<Vec<AlwaysAddEvent>> {
    prop::collection::vec(("[a-z ]{3,12}", source_strategy()), 1..4)
        .prop_map(|pairs| {
            let mut events = vec![];
            for (text, source) in &pairs {
                events.push(AlwaysAddEvent {
                    source: *source,
                    action: format!("always approve {text}"),
                });
                let parsed =
                    parse_untrusted_adds(&format!("page body: always approve {text}. keep going"));
                events.extend(parsed);
            }
            events
        })
        .boxed()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    #[test]
    fn t_safety_always_10(segments in prop::collection::vec(segment_strategy(), 1..6)) {
        let mut list = AlwaysList::default();
        let clock = FixedClock(1_700_000_000_000);
        let mut pressed: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        for segment in &segments {
            list.apply_events(segment, &clock);
            pressed.extend(
                segment
                    .iter()
                    .filter(|e| e.source.is_user_press())
                    .map(|e| e.action.clone()),
            );
        }
        let unique: std::collections::BTreeSet<String> =
            list.entries.iter().map(|e| e.action.clone()).collect();
        prop_assert_eq!(unique, pressed);
    }
}
