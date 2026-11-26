use std::sync::Mutex;

use anybot_foundation::ContentDigest;
use anybot_protocol::{BeforeDigest, ComputerFailure, TargetKind, WriteOutcome};
use anybot_trust::OpRejection;

use crate::fake_computer::FakeComputer;
use crate::fake_files::{FileTree, digest_text};

pub(crate) const TIMEOUT_EXIT_CODE: i32 = 124;
pub(crate) const NO_SCRIPT_REASON: &str = "the fake computer has no script for this command";
pub(crate) const MULTIPLE_REASON: &str = "the text to replace appears more than once in the file:";
pub(crate) const NOT_FOUND_REASON: &str = "the text to replace was not found in the file";
pub(crate) const MISSING_FILE_REASON: &str = "no such file on the fake computer";
pub(crate) const UNREACHABLE_REASON: &str = "the fake computer is set unreachable";

pub(crate) fn write_outcome(
    tree: &Mutex<FileTree>,
    path: &str,
    content: &str,
    before: Option<ContentDigest>,
) -> WriteOutcome {
    let canonical = path.to_owned();
    let outcome = WriteOutcome {
        path: canonical,
        before: match before {
            Some(digest) => BeforeDigest::Present(digest),
            None => BeforeDigest::Absent,
        },
        after: digest_text(content),
        size: content.len() as u64,
    };
    FakeComputer::guard(tree).write_exact(path, content);
    outcome
}

pub(crate) fn edit_rejection(rejection: &anybot_trust::EditRejection) -> OpRejection {
    match rejection {
        anybot_trust::EditRejection::NotFound => {
            OpRejection::from_failure(ComputerFailure::operation(NOT_FOUND_REASON.to_owned()))
        }
        anybot_trust::EditRejection::Multiple { hits } => OpRejection::from_failure(
            ComputerFailure::operation(format!("{MULTIPLE_REASON} {hits}")),
        ),
        anybot_trust::EditRejection::FileMissing => {
            OpRejection::from_failure(ComputerFailure::operation(MISSING_FILE_REASON.to_owned()))
        }
    }
}

pub(crate) fn apply_writes(tree: &Mutex<FileTree>, writes: &[(String, String)]) {
    let mut guard = FakeComputer::guard(tree);
    for (path, content) in writes {
        guard.write(path, content);
    }
}

pub(crate) fn target_kind(tree: &FileTree, path: &str) -> Option<TargetKind> {
    if tree.is_directory(path) {
        return Some(TargetKind::Directory);
    }
    if tree.exists(path) {
        return Some(TargetKind::File);
    }
    None
}
