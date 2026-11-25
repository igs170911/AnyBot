pub use anybot_protocol::TaskStatus;

#[must_use]
pub fn needs_attention(status: TaskStatus) -> bool {
    matches!(
        status,
        TaskStatus::AwaitingUser | TaskStatus::Stuck | TaskStatus::Failed
    )
}

#[cfg(test)]
mod tests {
    use super::TaskStatus;
    use super::needs_attention;

    #[test]
    fn stuck_and_failed_need_attention() {
        assert!(needs_attention(TaskStatus::Failed));
        assert!(needs_attention(TaskStatus::AwaitingUser));
        assert!(needs_attention(TaskStatus::Stuck));
        assert!(!needs_attention(TaskStatus::Running));
        assert!(!needs_attention(TaskStatus::Completed));
    }
}
