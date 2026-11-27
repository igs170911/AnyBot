use super::*;

fn event(name: Option<&str>, data: &str) -> SseEvent {
    SseEvent {
        name: name.map(str::to_owned),
        data: data.to_owned(),
    }
}

fn mark(stream: &str, seq: i64, device: &str) -> StreamMark {
    StreamMark {
        stream: stream.to_owned(),
        seq,
        device_id: device.to_owned(),
    }
}

#[test]
fn a_progress_event_lists_every_stream_with_its_last_record() {
    let data = r#"{"streams":[{"stream":"account/alice","seq":45,"device_id":"d1"},{"stream":"bot/b1","seq":3,"device_id":"d2"}]}"#;
    let hint = hint_of(&event(Some("progress"), data)).unwrap().unwrap();
    assert_eq!(
        hint,
        Hint::Progress {
            streams: vec![mark("account/alice", 45, "d1"), mark("bot/b1", 3, "d2")]
        }
    );
}

#[test]
fn the_closing_progress_event_reads_like_a_progress_event() {
    let data = r#"{"streams":[{"stream":"account/alice","seq":9,"device_id":"d1"}]}"#;
    let hint = hint_of(&event(Some("stream-progress"), data))
        .unwrap()
        .unwrap();
    assert_eq!(
        hint,
        Hint::Progress {
            streams: vec![mark("account/alice", 9, "d1")]
        }
    );
}

#[test]
fn a_new_records_event_names_the_stream_and_the_last_record() {
    let data = r#"{"stream":"account/alice","seq":46,"device_id":"d3"}"#;
    let hint = hint_of(&event(Some("new-records"), data)).unwrap().unwrap();
    assert_eq!(hint, Hint::News(mark("account/alice", 46, "d3")));
}

#[test]
fn unknown_and_unnamed_events_are_ignored_for_forward_compatibility() {
    assert_eq!(hint_of(&event(Some("task-state"), "{}")), Ok(None));
    assert_eq!(hint_of(&event(None, "hello")), Ok(None));
}

#[test]
fn unreadable_data_is_reported_not_swallowed() {
    let fault = hint_of(&event(Some("new-records"), "not json")).unwrap_err();
    let HintFault::Unreadable { event, .. } = fault;
    assert_eq!(event, "new-records");
}
