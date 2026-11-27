use std::collections::BTreeSet;
use std::sync::{Mutex, MutexGuard, PoisonError};

#[derive(Debug, Default)]
struct Sets {
    pulls: BTreeSet<String>,
    resyncs: BTreeSet<String>,
}

#[derive(Debug, Default)]
pub(crate) struct Wanted {
    sets: Mutex<Sets>,
}

pub(crate) enum Job {
    Pull(String),
    Resync(String),
}

impl Wanted {
    fn locked(&self) -> MutexGuard<'_, Sets> {
        self.sets.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub(crate) fn pull(&self, stream: &str) {
        self.locked().pulls.insert(stream.to_owned());
    }

    pub(crate) fn resync(&self, stream: &str) {
        let mut sets = self.locked();
        sets.pulls.remove(stream);
        sets.resyncs.insert(stream.to_owned());
    }

    pub(crate) fn next(&self) -> Option<Job> {
        let mut sets = self.locked();
        if let Some(stream) = sets.resyncs.pop_first() {
            return Some(Job::Resync(stream));
        }
        sets.pulls.pop_first().map(Job::Pull)
    }
}
