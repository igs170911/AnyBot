use crate::private_paths::is_private_with_caps;
use anybot_protocol::{ActualResult, ComputerCaps, DirEntry};

pub(crate) fn prune_private_entries(result: &mut ActualResult, caps: &ComputerCaps) {
    if let ActualResult::List(outcome) = result {
        outcome
            .entries
            .retain(|entry| !entry_is_private(&outcome.canonical, entry, caps));
    }
}

fn entry_is_private(canonical: &str, entry: &DirEntry, caps: &ComputerCaps) -> bool {
    is_private_with_caps(&child_path(canonical, &entry.name), caps, "")
}

fn child_path(canonical: &str, name: &str) -> String {
    let trimmed = canonical.trim_end_matches('/');
    if trimmed.is_empty() || trimmed == "/" {
        return format!("/{name}");
    }
    format!("{trimmed}/{name}")
}
