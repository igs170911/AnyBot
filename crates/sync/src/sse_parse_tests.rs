use super::*;

fn event(name: &str, data: &str) -> SseItem {
    SseItem::Event(SseEvent {
        name: Some(name.to_owned()),
        data: data.to_owned(),
    })
}

#[test]
fn events_and_heartbeats_come_out_in_order() {
    let mut parser = SseParser::new();
    let items =
        parser.push(b"event: progress\ndata: {\"a\":1}\n\n:hb\n\nevent: new-records\ndata: x\n\n");
    assert_eq!(
        items,
        vec![
            event("progress", "{\"a\":1}"),
            SseItem::Comment,
            event("new-records", "x")
        ]
    );
}

#[test]
fn any_split_of_the_bytes_gives_the_same_items() {
    let wire = b"event: a\ndata: first\n\n:hb\n\nevent: b\r\ndata: sec\r\ndata: ond\r\n\r\n";
    let mut whole = SseParser::new();
    let expected = whole.push(wire);
    assert_eq!(expected.len(), 3);
    for size in 1..wire.len() {
        let mut parser = SseParser::new();
        let mut got = Vec::new();
        for piece in wire.chunks(size) {
            got.extend(parser.push(piece));
        }
        assert_eq!(got, expected, "chunk size {size}");
    }
}

#[test]
fn multi_byte_text_split_between_chunks_survives() {
    let wire = "event: a\ndata: \u{85cd}\u{8272}\n\n".as_bytes();
    let mut parser = SseParser::new();
    let mut got = Vec::new();
    for piece in wire.chunks(1) {
        got.extend(parser.push(piece));
    }
    assert_eq!(got, vec![event("a", "\u{85cd}\u{8272}")]);
}

#[test]
fn an_unfinished_event_waits_for_its_blank_line() {
    let mut parser = SseParser::new();
    assert!(parser.push(b"event: a\ndata: partial").is_empty());
    assert!(parser.push(b"\n").is_empty());
    assert_eq!(parser.push(b"\n"), vec![event("a", "partial")]);
}

#[test]
fn data_only_events_have_no_name() {
    let mut parser = SseParser::new();
    let got = parser.push(b"data: hello\n\n");
    assert_eq!(
        got,
        vec![SseItem::Event(SseEvent {
            name: None,
            data: "hello".to_owned()
        })]
    );
}
