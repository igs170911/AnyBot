use crate::clock::Clock;
use crate::error::FoundationError;
use uuid::Uuid;

fn v7_bytes(now_ms: u64) -> [u8; 16] {
    let stamp = now_ms & (u64::MAX >> 16);
    let mut bytes = [0u8; 16];
    bytes[..6].copy_from_slice(&stamp.to_be_bytes()[2..]);
    rand::fill(&mut bytes[6..]);
    bytes[6] = 0x70 | (bytes[6] & 0x0F);
    bytes[8] = 0x80 | (bytes[8] & 0x3F);
    bytes
}

macro_rules! define_id {
    ($name:ident) => {
        #[derive(
            Debug,
            Clone,
            Copy,
            PartialEq,
            Eq,
            PartialOrd,
            Ord,
            Hash,
            serde::Serialize,
            serde::Deserialize,
        )]
        #[serde(transparent)]
        pub struct $name(Uuid);

        impl $name {
            #[must_use]
            pub fn generate(clock: &dyn Clock) -> Self {
                Self(Uuid::from_bytes(v7_bytes(clock.now_ms().millis())))
            }

            pub fn parse(text: &str) -> Result<Self, FoundationError> {
                Uuid::parse_str(text)
                    .map(Self)
                    .map_err(|e| FoundationError::IdParse(format!("id parse failed: {e}")))
            }

            #[must_use]
            pub fn as_str(&self) -> String {
                self.0.to_string()
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl std::str::FromStr for $name {
            type Err = FoundationError;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Self::parse(s)
            }
        }

        impl schemars::JsonSchema for $name {
            fn schema_name() -> std::borrow::Cow<'static, str> {
                std::borrow::Cow::Borrowed(stringify!($name))
            }

            fn schema_id() -> std::borrow::Cow<'static, str> {
                std::borrow::Cow::Borrowed(concat!("anybot_foundation::", stringify!($name)))
            }

            fn json_schema(
                generator: &mut schemars::generate::SchemaGenerator,
            ) -> schemars::Schema {
                <Uuid as schemars::JsonSchema>::json_schema(generator)
            }
        }
    };
}

define_id!(AccountId);
define_id!(DeviceId);
define_id!(BotId);
define_id!(TaskId);
define_id!(RecordId);
define_id!(TurnId);
define_id!(EventId);
define_id!(CardId);
define_id!(CallId);
define_id!(EvidenceId);
define_id!(BlobId);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::Clock;
    use crate::clock::TimestampMs;
    use std::collections::BTreeSet;

    struct SkewedClock(u64);

    impl Clock for SkewedClock {
        fn now_ms(&self) -> TimestampMs {
            TimestampMs::from_millis(self.0)
        }
    }

    #[test]
    fn task_ids_stay_unique_across_ten_skewed_clocks() {
        let base = 1_700_000_000_000u64;
        let mut seen = BTreeSet::new();
        for shift in 0..10 {
            let skew_ms: u64 = 3_600_000 * shift;
            let clock = SkewedClock(base.saturating_add(skew_ms.saturating_sub(3_600_000 * 5)));
            for _ in 0..100_000 {
                seen.insert(TaskId::generate(&clock));
            }
        }
        assert_eq!(seen.len(), 1_000_000);
    }

    #[test]
    fn test_ids_time_order_and_parse_roundtrip() {
        let early = TaskId::generate(&SkewedClock(1_700_000_000_000));
        let late = TaskId::generate(&SkewedClock(1_700_000_001_000));
        assert!(early < late);
        let text = late.as_str();
        assert_eq!(TaskId::parse(&text).ok(), Some(late));
        assert!(TaskId::parse("not-a-uuid").is_err());
    }

    #[test]
    fn test_ids_serialize_as_plain_string_and_time_ordered() {
        let a = AccountId::generate(&SkewedClock(1_700_000_000_000));
        let json = serde_json::to_string(&a).unwrap();
        assert_eq!(json, format!("\"{}\"", a.as_str()));
        assert_eq!(serde_json::from_str::<AccountId>(&json).unwrap(), a);
        let a2 = AccountId::generate(&SkewedClock(1_700_000_001_000));
        assert!(a < a2);
    }
}
