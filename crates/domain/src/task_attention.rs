use anybot_protocol::TaskStatus;

#[must_use]
pub fn counts_as_failure(status: TaskStatus) -> bool {
    status == TaskStatus::Failed
}

#[must_use]
pub fn counts_in_task_badge(status: TaskStatus) -> bool {
    !matches!(status, TaskStatus::Cancelled | TaskStatus::Stopped)
}

#[must_use]
pub fn badge_count(statuses: &[TaskStatus]) -> usize {
    statuses
        .iter()
        .copied()
        .filter(|s| counts_in_task_badge(*s))
        .count()
}

#[must_use]
pub fn routine_should_pause(consecutive: &[TaskStatus], limit: usize) -> bool {
    consecutive
        .iter()
        .take_while(|status| counts_as_failure(**status))
        .count()
        >= limit
}

#[cfg(test)]
#[path = "task_attention_tests.rs"]
mod tests;
