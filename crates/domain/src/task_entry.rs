use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntryInput {
    UserMessage,
    SteerMessage,
    RoutineFire,
    GoalSet,
    GoalNextRound,
    FollowUpWake,
}

impl EntryInput {
    #[must_use]
    pub fn opens_task(self) -> bool {
        matches!(
            self,
            EntryInput::UserMessage | EntryInput::RoutineFire | EntryInput::GoalSet
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct EntryCounts {
    pub user_messages: usize,
    pub routine_fires: usize,
    pub goal_sets: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntryTracker {
    pub tasks: usize,
    pub turns: usize,
    pub turns_per_task: Vec<usize>,
    pub current: Option<usize>,
    pub counts: EntryCounts,
}

impl EntryTracker {
    #[must_use]
    pub fn new() -> Self {
        EntryTracker {
            tasks: 0,
            turns: 0,
            turns_per_task: Vec::new(),
            current: None,
            counts: EntryCounts::default(),
        }
    }

    pub fn handle(&mut self, input: EntryInput) {
        self.turns = self.turns.saturating_add(1);
        if input.opens_task() {
            self.count_one(input);
            self.open_task();
            return;
        }
        self.join_current();
    }

    fn open_task(&mut self) {
        self.tasks = self.tasks.saturating_add(1);
        self.turns_per_task.push(1);
        self.current = self.tasks.checked_sub(1);
    }

    fn join_current(&mut self) {
        let Some(at) = self.current else {
            return;
        };
        let Some(slot) = self.turns_per_task.get_mut(at) else {
            return;
        };
        *slot = slot.saturating_add(1);
    }

    fn count_one(&mut self, input: EntryInput) {
        match input {
            EntryInput::UserMessage => {
                self.counts.user_messages = self.counts.user_messages.saturating_add(1);
            }
            EntryInput::RoutineFire => {
                self.counts.routine_fires = self.counts.routine_fires.saturating_add(1);
            }
            EntryInput::GoalSet => {
                self.counts.goal_sets = self.counts.goal_sets.saturating_add(1);
            }
            EntryInput::SteerMessage | EntryInput::GoalNextRound | EntryInput::FollowUpWake => {}
        }
    }

    #[must_use]
    pub fn task_count_matches_entries(&self) -> bool {
        let opens = self
            .counts
            .user_messages
            .saturating_add(self.counts.routine_fires)
            .saturating_add(self.counts.goal_sets);
        self.tasks == opens
    }

    #[must_use]
    pub fn every_turn_in_one_task(&self) -> bool {
        let sum: usize = self.turns_per_task.iter().sum();
        sum == self.turns
    }

    #[must_use]
    pub fn wakes_reuse_tasks(&self) -> bool {
        self.current.is_some() != self.turns_per_task.is_empty()
    }
}

impl Default for EntryTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[path = "task_entry_tests.rs"]
mod tests;
