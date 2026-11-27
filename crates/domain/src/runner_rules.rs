pub const RUNNER_NOTIFICATIONS_PATH: &str = "/v1/streams/notifications";
pub const RUNNER_MAILBOX_PREFIX: &str = "/v1/mailbox/";

#[must_use]
pub fn runner_may_call(path: &str) -> bool {
    path == RUNNER_NOTIFICATIONS_PATH || path.starts_with(RUNNER_MAILBOX_PREFIX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_own_stream_and_the_mailbox_are_open() {
        assert!(runner_may_call("/v1/streams/notifications"));
        assert!(runner_may_call("/v1/mailbox/claim"));
        assert!(!runner_may_call("/v1/admin/invite"));
        assert!(!runner_may_call("/v1/computers"));
        assert!(!runner_may_call("/v1/streams/after"));
        assert!(!runner_may_call("/v1/mailbox"));
    }
}
