#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountAction {
    DeliverRound,
    GeneratePairingCode,
    RemoveOtherComputer,
    ChangeSharing,
    PublishSite,
    ExportAccountData,
    RenameComputer,
    ReadComputerList,
}

pub const PAIRED_DENIED: [AccountAction; 6] = [
    AccountAction::DeliverRound,
    AccountAction::GeneratePairingCode,
    AccountAction::RemoveOtherComputer,
    AccountAction::ChangeSharing,
    AccountAction::PublishSite,
    AccountAction::ExportAccountData,
];

#[must_use]
pub fn paired_may(action: AccountAction) -> bool {
    !PAIRED_DENIED.contains(&action)
}

pub const RUNNER_ALLOWED_PATHS: [&str; 2] = ["/v1/streams/notifications", "/v1/mailbox/claim"];

#[must_use]
pub fn runner_may_call(path: &str) -> bool {
    RUNNER_ALLOWED_PATHS.contains(&path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paired_computers_lose_exactly_the_listed_actions() {
        for action in PAIRED_DENIED {
            assert!(!paired_may(action));
        }
        assert!(paired_may(AccountAction::RenameComputer));
        assert!(paired_may(AccountAction::ReadComputerList));
    }

    #[test]
    fn runner_service_reaches_only_its_two_routes() {
        assert!(runner_may_call("/v1/streams/notifications"));
        assert!(runner_may_call("/v1/mailbox/claim"));
        assert!(!runner_may_call("/v1/admin/invite"));
        assert!(!runner_may_call("/v1/computers/pairing-codes"));
    }
}
