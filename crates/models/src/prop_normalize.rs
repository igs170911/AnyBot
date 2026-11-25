use proptest::prelude::*;

use crate::prop_body::{ReplyCase, events_of, sse, whole_body};
use crate::prop_cut::{
    assert_column, cut_at, endpoint_for, plain_decode, same_answer, send_body, send_cut,
    stream_decode,
};

fn reply_strategy() -> impl Strategy<Value = ReplyCase> {
    (
        0_u8..3,
        prop_oneof![
            Just(String::new()),
            Just("ok".to_owned()),
            Just("\u{4e2d}\u{6587}".to_owned())
        ],
        prop_oneof![
            Just(String::new()),
            Just("plan".to_owned()),
            Just("\u{601d}\u{8003}".to_owned())
        ],
        prop_oneof![Just(String::new()), Just("read_file".to_owned())],
        prop_oneof![Just("{}".to_owned()), Just(r#"{"path":"a"}"#.to_owned())],
        prop_oneof![
            Just("stop".to_owned()),
            Just("tool_calls".to_owned()),
            Just("length".to_owned()),
            Just("refusal".to_owned()),
            Just("other".to_owned())
        ],
        prop_oneof![
            Just(String::new()),
            Just("kv_transfer_params".to_owned()),
            Just("token_ids".to_owned()),
            Just("metrics".to_owned()),
            Just("vendor_extra".to_owned())
        ],
        prop_oneof![
            Just(String::new()),
            Just("kept".to_owned()),
            Just("1".to_owned())
        ],
        any::<bool>(),
        0_usize..4_096,
    )
        .prop_map(
            |(format, text, thinking, tool, args, finish, extra_key, extra_value, stream, cut)| {
                ReplyCase {
                    format,
                    text,
                    thinking,
                    tool,
                    args,
                    finish,
                    extra_key,
                    extra_value,
                    stream,
                    cut,
                }
            },
        )
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    #[test]
    fn t_models_normalize_14(case in reply_strategy()) {
        let endpoint = endpoint_for(&case);
        let plain = whole_body(&case);
        let stream = sse(&events_of(&case), true);
        let bare = without_extra(&case);
        let sent = if case.stream { &stream } else { &plain };
        let (at, closed) = cut_at(sent, case.cut);
        let cut_result = if closed {
            send_body(&endpoint, sent, case.stream)
        } else {
            send_cut(&endpoint, sent, at, case.stream)
        };
        if !closed {
            let delivered = sent.as_bytes().get(..at).unwrap_or(&[]);
            prop_assert!(cut_matches(&cut_result, delivered, sent));
            prop_assert!(prefix_holds(&cut_result, delivered));
        }
        match cut_result {
            Ok(result) => assert_column(&result)?,
            Err(err) => prop_assert!(cut_failure(&err)),
        }
        let whole_stream = send_body(&endpoint, &stream, true).expect("stream send");
        let whole_plain = send_body(&endpoint, &plain, false).expect("plain send");
        prop_assert!(same_answer(&whole_stream, &whole_plain));
        let extra_free = if case.stream {
            send_body(&endpoint, &sse(&events_of(&bare), true), true)
        } else {
            send_body(&endpoint, &whole_body(&bare), false)
        }
        .expect("bare send");
        let with_extra = if case.stream { &whole_stream } else { &whole_plain };
        prop_assert!(same_answer(with_extra, &extra_free));
        prop_assert!(same_answer(&whole_plain, &plain_decode(&endpoint, &plain)));
        prop_assert!(same_answer(
            &whole_stream,
            &stream_decode(&endpoint, &stream).expect("stream decode")
        ));
    }
}

fn cut_matches(
    result: &Result<crate::types::TurnResult, crate::GatewayFail>,
    delivered: &[u8],
    whole: &str,
) -> bool {
    let shown = match result {
        Ok(result) => format!("{result:?}"),
        Err(err) => format!("{err:?}"),
    };
    let text = String::from_utf8_lossy(delivered);
    if text.as_ref() == whole {
        return false;
    }
    word_kept(whole, &text, &shown, "\u{4e2d}\u{6587}")
        && word_kept(whole, &text, &shown, "\u{601d}\u{8003}")
}

fn word_kept(whole: &str, delivered: &str, shown: &str, word: &str) -> bool {
    if !whole.contains(word) {
        return true;
    }
    let kept = word.chars().any(|unit| {
        let mut buffer = [0; 4];
        let encoded = unit.encode_utf8(&mut buffer);
        delivered.contains(&*encoded) && !delivered.contains(word)
    });
    if kept {
        return !shown.contains(word);
    }
    if delivered.contains(word) {
        return true;
    }
    !shown.contains(word)
}

fn prefix_holds(
    result: &Result<crate::types::TurnResult, crate::GatewayFail>,
    delivered: &[u8],
) -> bool {
    let Ok(result) = result else {
        return true;
    };
    let text = String::from_utf8_lossy(delivered);
    text.contains(result.text.as_str()) && text.contains(result.thinking.as_str())
}

fn cut_failure(err: &crate::GatewayFail) -> bool {
    let crate::GatewayFail::Classified { classified, .. } = err else {
        return false;
    };
    matches!(
        classified.kind,
        crate::errors::FailKind::FirstToken | crate::errors::FailKind::StreamCut
    )
}

fn without_extra(case: &ReplyCase) -> ReplyCase {
    let mut bare = case.clone();
    bare.extra_key.clear();
    bare.extra_value.clear();
    bare
}
