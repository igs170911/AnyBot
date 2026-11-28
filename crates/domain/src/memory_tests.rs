use anybot_foundation::{BotId, EvidenceId, RecordId, SystemClock, TaskId};

use crate::memory_admit::{Admission, MemoryClaimSource, MemoryProposal, admit};
use crate::memory_projection::project;
use crate::memory_types::{
    MemoryBook, MemoryEventContent, MemoryEventKind, MemoryEventRecord, MemoryEvidenceRef,
    MemoryKind, MemoryOrigin, MemoryScope, MemorySource, MemoryState,
};

pub(crate) fn fixed_bot() -> BotId {
    BotId::parse("00000000-0000-7000-8000-000000000001").unwrap()
}

pub(crate) fn tool_ref() -> MemoryEvidenceRef {
    MemoryEvidenceRef::TaskEvidence {
        task_id: TaskId::generate(&SystemClock),
        evidence_id: EvidenceId::generate(&SystemClock),
    }
}

pub(crate) fn base_proposal(id: &str, text: &str, claim: MemoryClaimSource) -> MemoryProposal {
    MemoryProposal {
        memory_id: id.to_owned(),
        scope: MemoryScope::Bot(fixed_bot()),
        kind: MemoryKind::Preference,
        origin: MemoryOrigin::Remembered,
        raw_content: text.to_owned(),
        claim,
        evidence_refs: Vec::new(),
        tags: vec!["ui".to_owned()],
        valid_from_ms: None,
        valid_to_ms: None,
        contradicts: None,
        reason: None,
    }
}

pub(crate) fn empty_event(
    seq: i64,
    at_ms: i64,
    kind: MemoryEventKind,
    id: &str,
    source: Option<MemorySource>,
) -> MemoryEventRecord {
    MemoryEventRecord {
        seq,
        created_at_ms: at_ms,
        event_kind: kind,
        content: MemoryEventContent {
            memory_id: id.to_owned(),
            version: 1,
            prev_version: None,
            kind: None,
            origin: None,
            content: None,
            source,
            evidence_refs: Vec::new(),
            scope: None,
            tags: Vec::new(),
            valid_from_ms: None,
            valid_to_ms: None,
            replaced_by: None,
            contradicts: None,
            reason: None,
        },
    }
}

#[test]
fn user_said_memory_is_confirmed() {
    let book = MemoryBook::default();
    let rec_id = RecordId::generate(&SystemClock);
    let proposal = base_proposal(
        "m-user",
        "I prefer dark mode",
        MemoryClaimSource::UserMessage {
            record_id: rec_id,
            message_text: "Actually,   I  PREFER  dark   mode in the app.".to_owned(),
        },
    );
    let Admission::Admit(admitted) = admit(&proposal, &book) else {
        panic!("expected admit");
    };
    assert_eq!(admitted.initial_state, MemoryState::Verified);
    assert_eq!(admitted.event_content.source, Some(MemorySource::UserSaid));
    let ev = MemoryEventRecord {
        seq: 1,
        created_at_ms: 1000,
        event_kind: admitted.event_kind,
        content: admitted.event_content,
    };
    let projected = project(&[ev]);
    let entry = projected.get("m-user").unwrap();
    assert_eq!(entry.state, MemoryState::Verified);
    assert!(!entry.is_candidate());

    let ungrounded = base_proposal(
        "m-fake-user",
        "User loves spicy food",
        MemoryClaimSource::UserMessage {
            record_id: rec_id,
            message_text: "Please open the calendar.".to_owned(),
        },
    );
    let Admission::Admit(downgraded) = admit(&ungrounded, &book) else {
        panic!("expected admit");
    };
    assert_eq!(downgraded.initial_state, MemoryState::Candidate);
    assert_eq!(
        downgraded.event_content.source,
        Some(MemorySource::ModelInference)
    );
}

#[test]
fn model_inference_starts_as_candidate() {
    let book = MemoryBook::default();
    for (id, claim) in [
        ("m-model", MemoryClaimSource::ModelInference),
        (
            "m-other",
            MemoryClaimSource::OtherBot {
                evidence_refs: vec![tool_ref()],
            },
        ),
        ("m-import", MemoryClaimSource::Import),
        (
            "m-empty-tool",
            MemoryClaimSource::ToolEvidence {
                evidence_refs: Vec::new(),
            },
        ),
    ] {
        let Admission::Admit(admitted) = admit(&base_proposal(id, id, claim), &book) else {
            panic!("expected admit for {id}");
        };
        assert_eq!(admitted.initial_state, MemoryState::Candidate);
        let projected = project(&[MemoryEventRecord {
            seq: 1,
            created_at_ms: 100,
            event_kind: admitted.event_kind,
            content: admitted.event_content,
        }]);
        assert!(projected.get(id).unwrap().is_candidate());
    }

    let Admission::Admit(supported) = admit(
        &base_proposal(
            "m-tool",
            "Deployed build 42",
            MemoryClaimSource::ToolEvidence {
                evidence_refs: vec![tool_ref()],
            },
        ),
        &book,
    ) else {
        panic!("expected admit");
    };
    assert_eq!(supported.initial_state, MemoryState::Supported);
}

fn blocked_upgrade_events(id: &str) -> Vec<MemoryEventRecord> {
    vec![
        empty_event(
            2,
            200,
            MemoryEventKind::Confirmed,
            id,
            Some(MemorySource::ModelInference),
        ),
        empty_event(
            3,
            300,
            MemoryEventKind::Confirmed,
            id,
            Some(MemorySource::OtherBot),
        ),
        empty_event(
            4,
            400,
            MemoryEventKind::Confirmed,
            id,
            Some(MemorySource::Import),
        ),
        empty_event(
            5,
            500,
            MemoryEventKind::Confirmed,
            id,
            Some(MemorySource::ToolEvidence),
        ),
        empty_event(6, 600, MemoryEventKind::Pinned, id, None),
    ]
}

#[test]
fn candidate_upgrades_only_by_confirm_or_tool_evidence() {
    let book = MemoryBook::default();
    let Admission::Admit(a1) = admit(
        &base_proposal(
            "m-cand",
            "Likes green tea",
            MemoryClaimSource::ModelInference,
        ),
        &book,
    ) else {
        panic!("expected admit");
    };
    let mut events = vec![MemoryEventRecord {
        seq: 1,
        created_at_ms: 100,
        event_kind: a1.event_kind,
        content: a1.event_content,
    }];
    events.extend(blocked_upgrade_events("m-cand"));
    let after_blocked = project(&events);
    let cand = after_blocked.get("m-cand").unwrap();
    assert_eq!(cand.state, MemoryState::Candidate);
    assert!(cand.pinned);

    let mut tool_confirm = empty_event(
        7,
        700,
        MemoryEventKind::Confirmed,
        "m-cand",
        Some(MemorySource::ToolEvidence),
    );
    tool_confirm.content.evidence_refs = vec![tool_ref()];
    events.push(tool_confirm);
    assert_eq!(
        project(&events).get("m-cand").unwrap().state,
        MemoryState::Supported
    );

    events.push(empty_event(
        8,
        800,
        MemoryEventKind::Confirmed,
        "m-cand",
        Some(MemorySource::UserSaid),
    ));
    assert_eq!(
        project(&events).get("m-cand").unwrap().state,
        MemoryState::Verified
    );
}

#[test]
fn contradiction_keeps_both_with_reason() {
    let mut initial = base_proposal("m-old", "Lives in Taipei", MemoryClaimSource::OwnerDirect);
    initial.scope = MemoryScope::AboutYou;
    let Admission::Admit(first) = admit(&initial, &MemoryBook::default()) else {
        panic!("expected admit");
    };
    let ev1 = MemoryEventRecord {
        seq: 1,
        created_at_ms: 1000,
        event_kind: first.event_kind,
        content: first.event_content,
    };
    let book1 = project(std::slice::from_ref(&ev1));
    let mut second_prop = base_proposal(
        "m-new",
        "Moved to Kaohsiung",
        MemoryClaimSource::ModelInference,
    );
    second_prop.scope = MemoryScope::AboutYou;
    second_prop.contradicts = Some("m-old".to_owned());
    second_prop.reason = Some("relocated".to_owned());
    let Admission::Admit(second) = admit(&second_prop, &book1) else {
        panic!("expected admit");
    };
    let ev2 = MemoryEventRecord {
        seq: 2,
        created_at_ms: 2000,
        event_kind: second.event_kind,
        content: second.event_content,
    };
    let book2 = project(&[ev1.clone(), ev2.clone()]);
    let old_entry = book2.get("m-old").unwrap();
    let new_entry = book2.get("m-new").unwrap();
    assert_eq!(old_entry.state, MemoryState::Contradicted);
    assert_eq!(new_entry.state, MemoryState::Contradicted);
    assert!(old_entry.is_active() && new_entry.is_active());
    assert_eq!(old_entry.contradictions.first().unwrap().other_id, "m-new");
    assert_eq!(
        old_entry.contradictions.first().unwrap().reason.as_deref(),
        Some("relocated")
    );
    assert_eq!(new_entry.contradictions.first().unwrap().other_id, "m-old");

    let confirm_new = empty_event(3, 3000, MemoryEventKind::Confirmed, "m-new", None);
    let book3 = project(&[ev1, ev2, confirm_new]);
    assert_eq!(book3.get("m-new").unwrap().state, MemoryState::Verified);
    assert_eq!(book3.get("m-old").unwrap().state, MemoryState::Outdated);
}
