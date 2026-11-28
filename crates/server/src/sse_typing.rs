use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::broadcast;

use anybot_bus::TypingNote;
use anybot_foundation::Clock;
use anybot_protocol::{SSE_EVENT_TYPING, TypingHint};

use crate::api_streams_sse_drive::json_line;

pub const TYPING_MIN_GAP_MS: u64 = 1_000;

#[derive(Default)]
pub struct TypingGate {
    last_sent_ms: HashMap<String, u64>,
    waiting: HashMap<String, TypingNote>,
}

impl TypingGate {
    pub fn offer(&mut self, note: TypingNote, now_ms: u64) -> Option<TypingNote> {
        let task = note.task_id.clone();
        let ready = self
            .last_sent_ms
            .get(&task)
            .is_none_or(|last| now_ms.saturating_sub(*last) >= TYPING_MIN_GAP_MS);
        if ready {
            self.waiting.remove(&task);
            self.last_sent_ms.insert(task, now_ms);
            return Some(note);
        }
        self.waiting.insert(task, note);
        None
    }

    #[must_use]
    pub fn next_wait(&self, now_ms: u64) -> Option<Duration> {
        self.waiting
            .keys()
            .filter_map(|task| self.last_sent_ms.get(task))
            .map(|last| {
                last.saturating_add(TYPING_MIN_GAP_MS)
                    .saturating_sub(now_ms)
            })
            .min()
            .map(Duration::from_millis)
    }

    pub fn take_due(&mut self, now_ms: u64) -> Vec<TypingNote> {
        let due: Vec<String> = self
            .waiting
            .keys()
            .filter(|task| self.is_due(task, now_ms))
            .cloned()
            .collect();
        let mut out = Vec::new();
        for task in due {
            if let Some(note) = self.waiting.remove(&task) {
                self.last_sent_ms.insert(task, now_ms);
                out.push(note);
            }
        }
        out
    }

    fn is_due(&self, task: &str, now_ms: u64) -> bool {
        self.last_sent_ms
            .get(task)
            .is_none_or(|last| now_ms.saturating_sub(*last) >= TYPING_MIN_GAP_MS)
    }
}

pub struct TypingFeed {
    rx: broadcast::Receiver<TypingNote>,
    keys: Vec<String>,
    clock: Arc<dyn Clock>,
    gate: TypingGate,
}

impl TypingFeed {
    #[must_use]
    pub fn new(
        rx: broadcast::Receiver<TypingNote>,
        keys: Vec<String>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        TypingFeed {
            rx,
            keys,
            clock,
            gate: TypingGate::default(),
        }
    }

    pub async fn next_lines(&mut self) -> Vec<String> {
        let now = self.clock.now_ms().millis();
        let wait = self.gate.next_wait(now);
        tokio::select! {
            note = self.rx.recv() => match note {
                Ok(note) => self.on_note(note),
                Err(broadcast::error::RecvError::Lagged(_skipped)) => Vec::new(),
                Err(broadcast::error::RecvError::Closed) => std::future::pending().await,
            },
            () = sleep_or_forever(&*self.clock, wait) => self.on_due(),
        }
    }

    fn on_note(&mut self, note: TypingNote) -> Vec<String> {
        if !self.keys.contains(&note.stream) {
            return Vec::new();
        }
        let now = self.clock.now_ms().millis();
        self.gate
            .offer(note, now)
            .and_then(|sent| hint_line(&sent))
            .into_iter()
            .collect()
    }

    fn on_due(&mut self) -> Vec<String> {
        let now = self.clock.now_ms().millis();
        self.gate
            .take_due(now)
            .iter()
            .filter_map(hint_line)
            .collect()
    }
}

async fn sleep_or_forever(clock: &dyn Clock, wait: Option<Duration>) {
    match wait {
        Some(wait) => clock.sleep(wait).await,
        None => std::future::pending().await,
    }
}

fn hint_line(note: &TypingNote) -> Option<String> {
    let hint = TypingHint {
        stream: note.stream.clone(),
        task_id: note.task_id.clone(),
        text: note.text.clone(),
        thinking: note.thinking.clone(),
        at_ms: i64::try_from(note.at.millis()).ok()?,
    };
    json_line(SSE_EVENT_TYPING, &hint).ok()
}

#[cfg(test)]
mod tests {
    use anybot_foundation::TimestampMs;

    use super::*;

    fn note(task: &str, text: &str) -> TypingNote {
        TypingNote {
            stream: "bot/b1".to_owned(),
            task_id: task.to_owned(),
            text: text.to_owned(),
            thinking: String::new(),
            at: TimestampMs::from_millis(0),
        }
    }

    #[test]
    fn first_note_of_a_task_goes_out_at_once() {
        let mut gate = TypingGate::default();
        assert_eq!(gate.offer(note("t", "a"), 5_000), Some(note("t", "a")));
        assert_eq!(gate.next_wait(5_000), None);
    }

    #[test]
    fn notes_inside_the_gap_collapse_to_the_latest() {
        let mut gate = TypingGate::default();
        gate.offer(note("t", "a"), 0);
        assert_eq!(gate.offer(note("t", "ab"), 100), None);
        assert_eq!(gate.offer(note("t", "abc"), 900), None);
        assert_eq!(gate.next_wait(900), Some(Duration::from_millis(100)));
        assert!(gate.take_due(999).is_empty());
        assert_eq!(gate.take_due(1_000), vec![note("t", "abc")]);
        assert_eq!(gate.next_wait(1_000), None);
    }

    #[test]
    fn note_after_the_gap_is_not_delayed() {
        let mut gate = TypingGate::default();
        gate.offer(note("t", "a"), 0);
        assert_eq!(gate.offer(note("t", "ab"), 1_000), Some(note("t", "ab")));
    }

    #[test]
    fn tasks_are_gated_separately() {
        let mut gate = TypingGate::default();
        gate.offer(note("one", "a"), 0);
        assert_eq!(gate.offer(note("two", "x"), 10), Some(note("two", "x")));
        assert_eq!(gate.offer(note("one", "ab"), 20), None);
        assert_eq!(gate.next_wait(20), Some(Duration::from_millis(980)));
    }
}
