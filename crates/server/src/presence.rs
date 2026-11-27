use std::collections::HashMap;
use std::sync::{Mutex, PoisonError};

use anybot_domain::computer_presence::write_due;

#[derive(Debug, Clone, Copy)]
struct Seen {
    last_ms: u64,
    written_ms: u64,
}

pub struct PresenceBook {
    started_ms: u64,
    seen: Mutex<HashMap<String, Seen>>,
}

impl PresenceBook {
    #[must_use]
    pub fn new(started_ms: u64) -> Self {
        PresenceBook {
            started_ms,
            seen: Mutex::new(HashMap::new()),
        }
    }

    #[must_use]
    pub fn started_ms(&self) -> u64 {
        self.started_ms
    }

    pub fn note_request(&self, device_id: &str, stored_ms: u64, now_ms: u64) -> bool {
        let mut seen = self.seen.lock().unwrap_or_else(PoisonError::into_inner);
        let entry = seen.entry(device_id.to_owned()).or_insert(Seen {
            last_ms: now_ms,
            written_ms: stored_ms,
        });
        entry.last_ms = now_ms;
        write_due(entry.written_ms, now_ms)
    }

    pub fn note_written(&self, device_id: &str, written_ms: u64) {
        let mut seen = self.seen.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(entry) = seen.get_mut(device_id) {
            entry.written_ms = entry.written_ms.max(written_ms);
        }
    }

    #[must_use]
    pub fn last_request(&self, device_id: &str) -> Option<u64> {
        let seen = self.seen.lock().unwrap_or_else(PoisonError::into_inner);
        seen.get(device_id).map(|entry| entry.last_ms)
    }

    #[must_use]
    pub fn unwritten(&self) -> Vec<(String, u64)> {
        let seen = self.seen.lock().unwrap_or_else(PoisonError::into_inner);
        seen.iter()
            .filter(|(_, entry)| entry.last_ms > entry.written_ms)
            .map(|(device, entry)| (device.clone(), entry.last_ms))
            .collect()
    }

    pub fn forget(&self, device_id: &str) {
        let mut seen = self.seen.lock().unwrap_or_else(PoisonError::into_inner);
        seen.remove(device_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_are_due_only_after_the_throttle() {
        let book = PresenceBook::new(0);
        assert!(!book.note_request("a", 1_000, 2_000));
        assert!(!book.note_request("a", 1_000, 10_999));
        assert!(book.note_request("a", 1_000, 11_000));
        book.note_written("a", 11_000);
        assert!(!book.note_request("a", 11_000, 12_000));
    }

    #[test]
    fn unwritten_lists_only_newer_requests() {
        let book = PresenceBook::new(0);
        book.note_request("a", 1_000, 2_000);
        assert_eq!(book.unwritten(), vec![("a".to_owned(), 2_000)]);
        book.note_written("a", 2_000);
        assert!(book.unwritten().is_empty());
        book.forget("a");
        assert_eq!(book.last_request("a"), None);
    }
}
