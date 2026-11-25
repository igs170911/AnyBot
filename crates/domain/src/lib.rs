pub use anybot_protocol::TaskStatus;

#[must_use]
pub fn needs_attention(status: TaskStatus) -> bool {
    matches!(status, TaskStatus::AwaitingApproval | TaskStatus::Failed)
}

#[cfg(test)]
mod tests {
    use super::TaskStatus;
    use super::needs_attention;

    #[test]
    fn stuck_and_failed_need_attention() {
        assert!(needs_attention(TaskStatus::Failed));
        assert!(needs_attention(TaskStatus::AwaitingApproval));
        assert!(!needs_attention(TaskStatus::Running));
    }
}
