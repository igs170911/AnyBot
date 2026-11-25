use anybot_foundation as _;
use bytes as _;
use futures_util as _;
use proptest::prelude::*;
use reqwest as _;
use serde as _;
use serde_json as _;
use test_harness as _;
use tokio as _;
use url as _;

use anybot_models::{
    Fixture, StopKind, WireFormat, closed_tool_matches, cut_has_no_tools, normalize_pair,
};

fn fixture_strategy() -> impl Strategy<Value = Fixture> {
    (
        0_u8..3,
        prop_oneof![
            Just(String::new()),
            Just("ok".to_owned()),
            Just("plan".to_owned())
        ],
        prop_oneof![Just(String::new()), Just("think".to_owned())],
        prop_oneof![Just(String::new()), Just("read_file".to_owned())],
        prop_oneof![Just("{}".to_owned()), Just(r#"{"path":"a"}"#.to_owned())],
        prop_oneof![
            Just("stop".to_owned()),
            Just("tool_calls".to_owned()),
            Just("length".to_owned())
        ],
        any::<bool>(),
        prop_oneof![Just(String::new()), Just("extra".to_owned())],
    )
        .prop_map(
            |(format, text, thinking, tool_name, tool_args, finish, cut, extra)| Fixture {
                format: match format {
                    0 | 1 => WireFormat::OpenAiCompatible,
                    _ => WireFormat::Anthropic,
                },
                text,
                thinking,
                tool_name,
                tool_args,
                finish,
                cut,
                extra,
            },
        )
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    #[test]
    fn stream_and_whole_decode_agree(fixture in fixture_strategy()) {
        let (whole, partial) = normalize_pair(&fixture);
        prop_assert!(closed_tool_matches(&whole));
        prop_assert!(cut_has_no_tools(&partial));
        if fixture.cut {
            prop_assert_eq!(partial.stop, StopKind::Interrupted);
            prop_assert!(partial.tool_calls.is_empty());
        } else {
            prop_assert_eq!(whole.stop, partial.stop);
            prop_assert_eq!(whole.text, partial.text);
            prop_assert_eq!(whole.thinking, partial.thinking);
            prop_assert_eq!(whole.tool_calls.len(), partial.tool_calls.len());
        }
        if !fixture.tool_name.is_empty() && !fixture.cut {
            prop_assert_eq!(whole.stop, StopKind::NeedsTool);
            prop_assert!(!whole.tool_calls.is_empty());
        }
        if fixture.tool_name.is_empty() && fixture.finish != "length" && !fixture.cut {
            prop_assert_eq!(whole.stop, StopKind::Finished);
            prop_assert!(whole.tool_calls.is_empty());
        }
    }
}
