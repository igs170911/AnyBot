use anybot_foundation::{BotId, RecordId, SystemClock};

use super::*;

#[test]
fn memory_event_kinds_match_account_stream_names() {
    let pairs = [
        (MemoryEventKind::Added, MEMORY_ADDED_KIND),
        (MemoryEventKind::Superseded, MEMORY_SUPERSEDED_KIND),
        (MemoryEventKind::Retracted, MEMORY_RETRACTED_KIND),
        (MemoryEventKind::Confirmed, MEMORY_CONFIRMED_KIND),
        (MemoryEventKind::Pinned, MEMORY_PINNED_KIND),
        (MemoryEventKind::Unpinned, MEMORY_UNPINNED_KIND),
        (MemoryEventKind::Voided, MEMORY_VOIDED_KIND),
        (MemoryEventKind::Restored, MEMORY_RESTORED_KIND),
    ];
    for (kind, expected) in pairs {
        assert_eq!(kind.kind_name(), expected);
        assert_eq!(MemoryEventKind::from_kind_name(expected), Some(kind));
        assert!(MEMORY_RECORD_KINDS.contains(&expected));
    }
    assert_eq!(MemoryEventKind::from_kind_name("message"), None);
}

#[test]
fn memory_event_content_roundtrips_json() {
    let bot = BotId::generate(&SystemClock);
    let rec = RecordId::generate(&SystemClock);
    let item = MemoryEventContent {
        memory_id: "01900000-0000-7000-8000-000000000010".to_owned(),
        version: 1,
        prev_version: None,
        kind: Some(MemoryKind::Preference),
        origin: Some(MemoryOrigin::Remembered),
        content: Some("prefers dark roast coffee".to_owned()),
        source: Some(MemorySource::UserSaid),
        evidence_refs: vec![MemoryEvidenceRef::UserRecord { record_id: rec }],
        scope: Some(MemoryScope::Bot(bot)),
        tags: vec!["coffee".to_owned()],
        valid_from_ms: Some(1_700_000_000_000),
        valid_to_ms: None,
        replaced_by: None,
        contradicts: None,
        reason: None,
    };
    let encoded = serde_json::to_string(&item).unwrap();
    let decoded: MemoryEventContent = serde_json::from_str(&encoded).unwrap();
    assert_eq!(decoded, item);
}
