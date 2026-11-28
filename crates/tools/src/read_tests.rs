use super::*;

#[test]
fn test_read_numbers_lines_from_the_requested_offset() {
    let out = numbered_lines("a\nb\nc\nd", 1, 2);
    assert_eq!(out.from, 2);
    assert_eq!(out.to, 4);
    assert_eq!(out.lines, vec!["b".to_owned(), "c".to_owned()]);
    assert_eq!(out.remaining, 1);
    assert_eq!(out.next_offset, 3);
}

#[test]
fn test_read_blocks_are_reported_for_size_directory_and_binary() {
    assert_eq!(
        block_of(MAX_READ_BYTES.saturating_add(1), false, "text"),
        Some(ReadBlock::TooBig {
            size: MAX_READ_BYTES.saturating_add(1),
        })
    );
    assert_eq!(block_of(10, true, ""), Some(ReadBlock::Directory));
    assert_eq!(block_of(10, false, "a\u{0}b"), Some(ReadBlock::NotText));
    assert_eq!(block_of(10, false, "plain"), None);
}

#[test]
fn test_read_limit_takes_the_requested_window() {
    let out = numbered_lines("a\nb\nc", 0, 2);
    assert_eq!(out.lines.len(), 2);
    assert_eq!(out.to, 3);
    assert_eq!(out.remaining, 1);
    assert_eq!(out.next_offset, 2);
}

#[test]
fn read_limit_is_exactly_8_mib() {
    assert_eq!(MAX_READ_BYTES, 8_388_608);
    assert_eq!(block_of(8_388_608, false, "plain"), None);
    assert_eq!(
        block_of(8_388_609, false, "plain"),
        Some(ReadBlock::TooBig { size: 8_388_609 })
    );
}

#[test]
fn nul_probe_stops_at_8000_bytes() {
    let mut nul_at_8000 = "a".repeat(7_999);
    nul_at_8000.push('\u{0}');
    nul_at_8000.push_str("tail");
    assert_eq!(
        block_of(8_004, false, &nul_at_8000),
        Some(ReadBlock::NotText)
    );

    let mut nul_at_8001 = "a".repeat(8_000);
    nul_at_8001.push('\u{0}');
    nul_at_8001.push_str("tail");
    assert_eq!(block_of(8_005, false, &nul_at_8001), None);
}

#[test]
fn offset_continues_where_hint_says() {
    let three_k: Vec<String> = (1..=3_000).map(|n| format!("line-{n}")).collect();
    let content_3k = three_k.join("\n");
    let first = numbered_lines(&content_3k, 0, DEFAULT_READ_LIMIT);
    assert_eq!(first.lines.len(), 2_000);
    assert_eq!(first.remaining, 1_000);
    assert_eq!(first.next_offset, 2_000);

    let second = numbered_lines(&content_3k, first.next_offset, DEFAULT_READ_LIMIT);
    assert_eq!(second.from, 2_001);
    assert_eq!(second.lines.len(), 1_000);
    assert_eq!(second.lines.first().map(String::as_str), Some("line-2001"));
    assert_eq!(second.lines.last().map(String::as_str), Some("line-3000"));
    assert_eq!(second.remaining, 0);

    let two_k: Vec<String> = (1..=2_000).map(|n| format!("line-{n}")).collect();
    let exact = numbered_lines(&two_k.join("\n"), 0, DEFAULT_READ_LIMIT);
    assert_eq!(exact.lines.len(), 2_000);
    assert_eq!(exact.remaining, 0);
}
