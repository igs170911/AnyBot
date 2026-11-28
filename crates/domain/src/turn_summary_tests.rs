use anybot_foundation::{ContentDigest, SystemClock, TaskId, TimestampMs, TurnId};
use anybot_protocol::{
    ActionOutcome, EvidenceKind, EvidencePayload, EvidenceRecord, EvidenceShortId, GateOutcome,
    GateRecord, GateStep, JudgementSource, KnownTaskEventKind, RuleLayer, TaskEvent, TaskEventKind,
    TurnEndReason,
};

use crate::turn_summary::{SummaryError, turn_summary};

fn ev(at_ms: u64, kind: KnownTaskEventKind) -> TaskEvent {
    TaskEvent {
        event_id: anybot_foundation::EventId::generate(&SystemClock),
        seq: 1,
        at_ms: TimestampMs::from_millis(at_ms),
        kind: TaskEventKind::Known(kind),
    }
}

fn gate() -> GateRecord {
    GateRecord::new(
        GateStep::DefaultRule,
        RuleLayer::Default,
        "bot-machine-files".to_owned(),
        JudgementSource::Rule,
        GateOutcome::Allow,
        true,
    )
}

fn read_record(task: TaskId, turn: TurnId, at: u64, path: &str) -> EvidenceRecord {
    EvidenceRecord {
        id: anybot_foundation::EvidenceId::generate(&SystemClock),
        short: EvidenceShortId(1),
        at: TimestampMs::from_millis(at),
        task,
        turn,
        computer: "bot-1".to_owned(),
        workdir: "/workspace".to_owned(),
        tool: "read_file".to_owned(),
        classes: Vec::new(),
        gate: gate(),
        kind: EvidenceKind::Seen,
        outcome: ActionOutcome::Completed,
        payload: EvidencePayload::Seen {
            path: path.to_owned(),
            digest: ContentDigest::sha256(path.as_bytes()),
            size: 1,
        },
        verification: None,
        source: JudgementSource::Rule,
        ungated: false,
    }
}

#[test]
fn turn_summary_dedups_caps_twelve_and_tracks_running_now() {
    let task = TaskId::generate(&SystemClock);
    let turn = TurnId::generate(&SystemClock);
    let mut events = vec![ev(
        1_000,
        KnownTaskEventKind::ToolStarted {
            call_id: anybot_foundation::CallId::generate(&SystemClock),
        },
    )];
    for i in 0..14 {
        let rec = read_record(task, turn, 1_100 + i, &format!("f{i}.txt"));
        events.push(ev(
            1_100 + i,
            KnownTaskEventKind::EvidenceAdded {
                record: Box::new(rec),
            },
        ));
    }
    let dup = read_record(task, turn, 1_200, "f0.txt");
    events.push(ev(
        1_200,
        KnownTaskEventKind::EvidenceAdded {
            record: Box::new(dup),
        },
    ));
    let running = turn_summary(&events, &turn, TimestampMs::from_millis(2_500)).unwrap();
    assert_eq!(running.seen.len(), 12);
    assert_eq!(running.elapsed_ms, 1_500);
    assert_eq!(running.tool_counts, vec![("read_file".to_owned(), 15)]);
    events.push(ev(
        1_400,
        KnownTaskEventKind::TurnEnded {
            reason: TurnEndReason::Answered,
            reply_delivered: true,
        },
    ));
    let finished = turn_summary(&events, &turn, TimestampMs::from_millis(9_999)).unwrap();
    assert_eq!(finished.elapsed_ms, 200);
    assert_eq!(
        turn_summary(
            &events,
            &TurnId::generate(&SystemClock),
            TimestampMs::from_millis(0)
        ),
        Err(SummaryError::TurnNotFound)
    );
}
