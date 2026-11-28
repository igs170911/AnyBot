use super::*;

#[test]
fn t_server_n4_4() {
    let title = "t".repeat(100);
    let text = "x".repeat(300);
    let message = PushMessage::compose(&title, "card-ask", &[("text", &text)], "a b/c", 5);
    assert_eq!(message.title.chars().count(), 80);
    assert_eq!(
        message.body_params.get("text").map(|v| v.chars().count()),
        Some(200)
    );
    assert_eq!(message.tag, "abc");
}

#[test]
fn compose_caps_question_tag_and_badge() {
    let question = "q".repeat(150);
    let long_tag = "k".repeat(60);
    let message = PushMessage::compose("", "card-ask", &[("question", &question)], &long_tag, 5000);
    assert_eq!(message.title, DEFAULT_TITLE);
    assert_eq!(
        message
            .body_params
            .get("question")
            .map(|v| v.chars().count()),
        Some(100)
    );
    assert_eq!(message.tag.len(), TAG_MAX);
    assert_eq!(message.badge, 999);
    assert_eq!(message.topic().map(|t| t.len()), Some(TOPIC_MAX));
}

#[test]
fn cut_counts_characters_not_bytes() {
    let title = "\u{4e2d}".repeat(100);
    let message = PushMessage::compose(&title, "card-ask", &[], "", 0);
    assert_eq!(message.title.chars().count(), 80);
    assert_eq!(message.topic(), None);
}

#[test]
fn oversize_payload_is_refused() {
    let mut params = Vec::new();
    let value = "v".repeat(200);
    let names: Vec<String> = (0..30).map(|n| format!("p{n}")).collect();
    for name in &names {
        params.push((name.as_str(), value.as_str()));
    }
    let message = PushMessage::compose("t", "card-ask", &params, "tag", 1);
    assert!(matches!(
        message.to_bytes(),
        Err(PushError::PayloadTooLarge)
    ));
}
