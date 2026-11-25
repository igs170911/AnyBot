use serde::{Deserialize, Serialize};

use anybot_foundation::{SystemClock, TaskId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JournalOp {
    OpenTask,
    StateChange,
    EvidenceAdd,
    ClearChat,
    NewConversation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskJournal {
    pub tasks: Vec<TaskId>,
    pub seqs: Vec<u64>,
}

impl TaskJournal {
    #[must_use]
    pub fn new() -> Self {
        TaskJournal {
            tasks: Vec::new(),
            seqs: Vec::new(),
        }
    }

    pub fn apply(&mut self, op: &JournalOp) {
        match op {
            JournalOp::OpenTask => {
                self.tasks.push(TaskId::generate(&SystemClock));
                self.seqs.push(0);
            }
            JournalOp::StateChange | JournalOp::EvidenceAdd => {
                if let Some(last) = self.seqs.last_mut() {
                    *last = last.saturating_add(1);
                }
            }
            JournalOp::ClearChat | JournalOp::NewConversation => {}
        }
    }

    #[must_use]
    pub fn record_count(&self, task_index: usize) -> Option<usize> {
        self.seqs
            .get(task_index)
            .and_then(|n| usize::try_from(*n).ok())
    }

    #[must_use]
    pub fn open_task_count(&self) -> usize {
        self.tasks.len()
    }

    #[must_use]
    pub fn shape_matches(&self) -> bool {
        self.tasks.len() == self.seqs.len()
    }

    #[must_use]
    pub fn journal_intact_after_clear(&self, before: &TaskJournal) -> bool {
        self.tasks == before.tasks && self.seqs == before.seqs
    }
}

impl Default for TaskJournal {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[path = "task_journal_tests.rs"]
mod tests;
