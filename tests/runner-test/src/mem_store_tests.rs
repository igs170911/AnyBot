use std::sync::{Arc, Barrier};

use anybot_foundation::{BotId, EventId, TaskId, TimestampMs};
use anybot_protocol::{KnownTaskEventKind, TaskEventKind};
use anybot_runner::{Appended, Halt, NewTaskEvent, RecordEntry, StoredBody};
use test_harness::FakeClock;

use crate::mem_store::MemoryRecord;

fn task_entry(bot: BotId, task: TaskId, kind: KnownTaskEventKind, ms: u64) -> RecordEntry {
    let clock = FakeClock::new(ms);
    RecordEntry::Task(NewTaskEvent {
        bot,
        task,
        turn: Some(1),
        event_id: EventId::generate(&*clock),
        at_ms: TimestampMs::from_millis(ms),
        kind: TaskEventKind::Known(kind),
    })
}

#[test]
fn append_unless_halted_blocks_after_stop_and_cancel() {
    let clock = FakeClock::new(100);
    let bot = BotId::generate(&*clock);
    let task = TaskId::generate(&*clock);
    let record = MemoryRecord::new();

    let started = record
        .append_sync(task_entry(
            bot,
            task,
            KnownTaskEventKind::ModelAsked { step: 1 },
            100,
        ))
        .expect("started");
    assert_eq!(started.stream_seq, 1);

    let before_stop = record
        .append_unless_halted_sync(
            task_entry(bot, task, KnownTaskEventKind::ModelAsked { step: 2 }, 101),
            started.stream_seq,
        )
        .expect("ok");
    assert!(matches!(before_stop, Appended::Stored(_)));

    record
        .append_sync(task_entry(bot, task, KnownTaskEventKind::TaskStopped, 102))
        .expect("stop");

    let count_before = record.len();
    let after_stop = record
        .append_unless_halted_sync(
            task_entry(bot, task, KnownTaskEventKind::ModelAsked { step: 3 }, 103),
            started.stream_seq,
        )
        .expect("halted");
    assert_eq!(after_stop, Appended::Halted(Halt::Stopped));
    assert_eq!(record.len(), count_before);

    record
        .append_sync(task_entry(
            bot,
            task,
            KnownTaskEventKind::TaskCancelled {
                reason: "user".to_owned(),
            },
            104,
        ))
        .expect("cancel");

    let after_cancel = record
        .append_unless_halted_sync(
            task_entry(bot, task, KnownTaskEventKind::ModelAsked { step: 4 }, 105),
            started.stream_seq,
        )
        .expect("cancelled");
    assert_eq!(after_cancel, Appended::Halted(Halt::Cancelled));
    assert_eq!(record.len(), count_before + 1);
}

#[test]
fn append_unless_halted_ignores_prior_turn_stop() {
    let clock = FakeClock::new(100);
    let bot = BotId::generate(&*clock);
    let task = TaskId::generate(&*clock);
    let record = MemoryRecord::new();

    record
        .append_sync(task_entry(bot, task, KnownTaskEventKind::TaskStopped, 100))
        .expect("old stop");
    let turn2 = record
        .append_sync(task_entry(
            bot,
            task,
            KnownTaskEventKind::ModelAsked { step: 1 },
            101,
        ))
        .expect("turn2");

    let next = record
        .append_unless_halted_sync(
            task_entry(bot, task, KnownTaskEventKind::ModelAsked { step: 2 }, 102),
            turn2.stream_seq,
        )
        .expect("allowed");
    assert!(matches!(next, Appended::Stored(_)));
}

#[test]
fn append_unless_halted_atomic_in_same_critical_section() {
    for round in 0..50u64 {
        run_atomic_round(round);
    }
}

fn run_atomic_round(round: u64) {
    let clock = FakeClock::new(1_000_u64.saturating_add(round));
    let bot = BotId::generate(&*clock);
    let task = TaskId::generate(&*clock);
    let record = Arc::new(MemoryRecord::new());
    let turn_started = record
        .append_sync(task_entry(
            bot,
            task,
            KnownTaskEventKind::ModelAsked { step: 1 },
            1_000,
        ))
        .expect("turn")
        .stream_seq;
    let barrier = Arc::new(Barrier::new(2));
    std::thread::scope(|scope| {
        spawn_atomic_writer(scope, &record, &barrier, bot, task, turn_started);
        spawn_atomic_stopper(scope, &record, &barrier, bot, task);
    });
    assert_asked_before_stop(&record);
}

fn spawn_atomic_writer<'a>(
    scope: &'a std::thread::Scope<'a, '_>,
    record: &'a Arc<MemoryRecord>,
    barrier: &'a Arc<Barrier>,
    bot: BotId,
    task: TaskId,
    turn_started: u64,
) {
    let rec_w = Arc::clone(record);
    let bar_w = Arc::clone(barrier);
    scope.spawn(move || {
        bar_w.wait();
        for step in 2..80u32 {
            let res = rec_w
                .append_unless_halted_sync(
                    task_entry(bot, task, KnownTaskEventKind::ModelAsked { step }, 1_001),
                    turn_started,
                )
                .expect("append");
            if matches!(res, Appended::Halted(_)) {
                break;
            }
        }
    });
}

fn spawn_atomic_stopper<'a>(
    scope: &'a std::thread::Scope<'a, '_>,
    record: &'a Arc<MemoryRecord>,
    barrier: &'a Arc<Barrier>,
    bot: BotId,
    task: TaskId,
) {
    let rec_s = Arc::clone(record);
    let bar_s = Arc::clone(barrier);
    scope.spawn(move || {
        bar_s.wait();
        for _ in 0..10 {
            std::thread::yield_now();
        }
        rec_s
            .append_sync(task_entry(
                bot,
                task,
                KnownTaskEventKind::TaskStopped,
                1_002,
            ))
            .expect("stop");
    });
}

fn assert_asked_before_stop(record: &MemoryRecord) {
    let rows = record.snapshot();
    let stopped_seq = rows
        .iter()
        .find_map(|row| match &row.body {
            StoredBody::Task { event, .. }
                if matches!(
                    event.kind,
                    TaskEventKind::Known(KnownTaskEventKind::TaskStopped)
                ) =>
            {
                Some(row.stream_seq)
            }
            StoredBody::Task { .. } | StoredBody::UserSaid(_) => None,
        })
        .expect("stopped seq");
    for row in &rows {
        if let StoredBody::Task { event, .. } = &row.body
            && matches!(
                event.kind,
                TaskEventKind::Known(KnownTaskEventKind::ModelAsked { .. })
            )
        {
            assert!(row.stream_seq < stopped_seq);
        }
    }
}
