use proptest::prelude::*;

use anybot_foundation::{BotId, EvidenceId, TaskId};

use crate::memory_projection::project;
use crate::memory_types::{
    MemoryEventContent, MemoryEventKind, MemoryEventRecord, MemoryEvidenceRef, MemoryKind,
    MemoryOrigin, MemoryScope, MemorySource, MemoryState,
};

#[derive(Debug, Clone)]
enum StepOp {
    Confirm(Option<MemorySource>, bool),
    Pin,
    Unpin,
    Retract,
    Restore,
    EditByModel,
    Void,
}

fn step_op_strategy() -> BoxedStrategy<StepOp> {
    prop_oneof![
        (source_opt_strategy(), any::<bool>())
            .prop_map(|(src, with_ref)| StepOp::Confirm(src, with_ref)),
        Just(StepOp::Pin),
        Just(StepOp::Unpin),
        Just(StepOp::Retract),
        Just(StepOp::Restore),
        Just(StepOp::EditByModel),
        Just(StepOp::Void),
    ]
    .boxed()
}

fn source_strategy() -> BoxedStrategy<MemorySource> {
    prop_oneof![
        Just(MemorySource::UserSaid),
        Just(MemorySource::ToolEvidence),
        Just(MemorySource::ModelInference),
        Just(MemorySource::OtherBot),
        Just(MemorySource::Import),
    ]
    .boxed()
}

fn source_opt_strategy() -> BoxedStrategy<Option<MemorySource>> {
    prop_oneof![Just(None), source_strategy().prop_map(Some)].boxed()
}

fn fixed_bot() -> BotId {
    BotId::parse("00000000-0000-7000-8000-000000000001").unwrap()
}

fn fixed_tool_ref() -> MemoryEvidenceRef {
    MemoryEvidenceRef::TaskEvidence {
        task_id: TaskId::parse("00000000-0000-7000-8000-000000000010").unwrap(),
        evidence_id: EvidenceId::parse("00000000-0000-7000-8000-000000000020").unwrap(),
    }
}

fn build_stream(
    id: &str,
    init_source: MemorySource,
    init_has_ref: bool,
    ops: &[StepOp],
) -> Vec<MemoryEventRecord> {
    let mut events = Vec::with_capacity(ops.len().saturating_add(1));
    let refs = if init_has_ref {
        vec![fixed_tool_ref()]
    } else {
        Vec::new()
    };
    events.push(MemoryEventRecord {
        seq: 0,
        created_at_ms: 100,
        event_kind: MemoryEventKind::Added,
        content: MemoryEventContent {
            memory_id: id.to_owned(),
            version: 1,
            prev_version: None,
            kind: Some(MemoryKind::Knowledge),
            origin: Some(MemoryOrigin::Remembered),
            content: Some(format!("fact-{id}")),
            source: Some(init_source),
            evidence_refs: refs,
            scope: Some(MemoryScope::Bot(fixed_bot())),
            tags: Vec::new(),
            valid_from_ms: None,
            valid_to_ms: None,
            replaced_by: None,
            contradicts: None,
            reason: None,
        },
    });
    for op in ops {
        events.push(op_to_record(id, op));
    }
    events
}

fn op_to_record(id: &str, op: &StepOp) -> MemoryEventRecord {
    let (kind, source, refs, content) = match op {
        StepOp::Confirm(src, with_ref) => (
            MemoryEventKind::Confirmed,
            *src,
            if *with_ref {
                vec![fixed_tool_ref()]
            } else {
                Vec::new()
            },
            None,
        ),
        StepOp::Pin => (MemoryEventKind::Pinned, None, Vec::new(), None),
        StepOp::Unpin => (MemoryEventKind::Unpinned, None, Vec::new(), None),
        StepOp::Retract => (MemoryEventKind::Retracted, None, Vec::new(), None),
        StepOp::Restore => (MemoryEventKind::Restored, None, Vec::new(), None),
        StepOp::EditByModel => (
            MemoryEventKind::Superseded,
            Some(MemorySource::ModelInference),
            Vec::new(),
            Some(format!("edited-{id}")),
        ),
        StepOp::Void => (MemoryEventKind::Voided, None, Vec::new(), None),
    };
    MemoryEventRecord {
        seq: 0,
        created_at_ms: 200,
        event_kind: kind,
        content: MemoryEventContent {
            memory_id: id.to_owned(),
            version: 2,
            prev_version: None,
            kind: None,
            origin: None,
            content,
            source,
            evidence_refs: refs,
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

fn interleave(
    mut streams: Vec<Vec<MemoryEventRecord>>,
    choices: &[usize],
) -> Vec<MemoryEventRecord> {
    for stream in &mut streams {
        stream.reverse();
    }
    let mut out = Vec::new();
    let mut seq = 1i64;
    let mut at_ms = 1000i64;
    for &pick in choices {
        let len = streams.len();
        if len == 0 {
            break;
        }
        let Some(idx) = pick.checked_rem(len) else {
            break;
        };
        if let Some(mut ev) = streams.get_mut(idx).and_then(Vec::pop) {
            ev.seq = seq;
            ev.created_at_ms = at_ms;
            seq = seq.saturating_add(1);
            at_ms = at_ms.saturating_add(10);
            out.push(ev);
        }
    }
    for stream in &mut streams {
        while let Some(mut ev) = stream.pop() {
            ev.seq = seq;
            ev.created_at_ms = at_ms;
            seq = seq.saturating_add(1);
            at_ms = at_ms.saturating_add(10);
            out.push(ev);
        }
    }
    out
}

fn has_authorized_upgrade(init_src: MemorySource, init_ref: bool, ops: &[StepOp]) -> bool {
    let init_ok = matches!(init_src, MemorySource::UserSaid)
        || (init_src == MemorySource::ToolEvidence && init_ref);
    if init_ok {
        return true;
    }
    ops.iter().any(|op| {
        matches!(
            op,
            StepOp::Confirm(None | Some(MemorySource::UserSaid), _)
                | StepOp::Confirm(Some(MemorySource::ToolEvidence), true)
                | StepOp::Void
        )
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    #[test]
    fn projection_is_deterministic_under_any_interleaving(
        s0 in source_strategy(),
        r0 in any::<bool>(),
        ops0 in prop::collection::vec(step_op_strategy(), 0..12),
        s1 in source_strategy(),
        r1 in any::<bool>(),
        ops1 in prop::collection::vec(step_op_strategy(), 0..12),
        choices in prop::collection::vec(0usize..8, 0..40),
    ) {
        let stream0 = build_stream("m0", s0, r0, &ops0);
        let stream1 = build_stream("m1", s1, r1, &ops1);
        let interleaved = interleave(vec![stream0.clone(), stream1.clone()], &choices);

        let book_a = project(&interleaved);
        let book_b = project(&interleaved);
        prop_assert_eq!(&book_a, &book_b);

        let solo0 = project(&stream0);
        let solo1 = project(&stream1);
        prop_assert_eq!(
            book_a.get("m0").map(|e| (&e.content, e.state, e.pinned, e.retracted_at_ms.is_some())),
            solo0.get("m0").map(|e| (&e.content, e.state, e.pinned, e.retracted_at_ms.is_some())),
        );
        prop_assert_eq!(
            book_a.get("m1").map(|e| (&e.content, e.state, e.pinned, e.retracted_at_ms.is_some())),
            solo1.get("m1").map(|e| (&e.content, e.state, e.pinned, e.retracted_at_ms.is_some())),
        );

        if !has_authorized_upgrade(s0, r0, &ops0)
            && let Some(entry) = book_a.get("m0")
        {
            prop_assert_eq!(entry.state, MemoryState::Candidate);
        }
    }
}
