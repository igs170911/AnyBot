use anybot_foundation::ContentDigest;
use serde::{Deserialize, Serialize};

use crate::computer_caps::ComputerCaps;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandRequest {
    pub command: String,
    pub timeout_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandOutcome {
    pub command: String,
    pub computer: String,
    pub exit_code: Option<i32>,
    pub output: String,
    pub elapsed_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BeforeDigest {
    Absent,
    Present(ContentDigest),
}

impl BeforeDigest {
    #[must_use]
    pub fn digest(&self) -> Option<&ContentDigest> {
        match self {
            BeforeDigest::Absent => None,
            BeforeDigest::Present(value) => Some(value),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WriteRequest {
    pub path: String,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WriteOutcome {
    pub path: String,
    pub before: BeforeDigest,
    pub after: ContentDigest,
    pub size: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReadRequest {
    pub path: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetKind {
    File,
    Directory,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReadOutcome {
    pub path: String,
    pub kind: TargetKind,
    pub digest: ContentDigest,
    pub size: u64,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PathRequest {
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PathOutcome {
    pub path: String,
    pub canonical: String,
    pub kind: Option<TargetKind>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActualResult {
    Command(CommandOutcome),
    Write(WriteOutcome),
    Read(ReadOutcome),
    Path(PathOutcome),
    Caps(ComputerCaps),
}

impl ActualResult {
    #[must_use]
    pub fn computer_path(&self) -> Option<&str> {
        match self {
            ActualResult::Write(outcome) => Some(&outcome.path),
            ActualResult::Read(outcome) => Some(&outcome.path),
            ActualResult::Path(outcome) => Some(&outcome.path),
            ActualResult::Command(_) | ActualResult::Caps(_) => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComputerFailureKind {
    Unreachable,
    Operation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComputerFailure {
    pub kind: ComputerFailureKind,
    pub reason: String,
}

impl ComputerFailure {
    #[must_use]
    pub fn unreachable(reason: String) -> Self {
        ComputerFailure {
            kind: ComputerFailureKind::Unreachable,
            reason,
        }
    }

    #[must_use]
    pub fn operation(reason: String) -> Self {
        ComputerFailure {
            kind: ComputerFailureKind::Operation,
            reason,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::computer_caps::{ComputerOwner, ComputerPlatform};

    #[test]
    fn test_write_outcome_carries_before_after_and_size() {
        let outcome = WriteOutcome {
            path: "/workspace/notes.md".to_owned(),
            before: BeforeDigest::Absent,
            after: ContentDigest::sha256(b"hello"),
            size: 5,
        };
        assert_eq!(outcome.before.digest(), None);
        assert_eq!(outcome.size, 5);
    }

    #[test]
    fn test_actual_result_exposes_paths_only_for_file_results() {
        let read = ActualResult::Read(ReadOutcome {
            path: "/workspace/a".to_owned(),
            kind: TargetKind::File,
            digest: ContentDigest::sha256(b"a"),
            size: 1,
            content: "a".to_owned(),
        });
        let caps = ActualResult::Caps(ComputerCaps::declared(
            ComputerPlatform::Linux,
            ComputerOwner::Bot,
        ));
        assert_eq!(read.computer_path(), Some("/workspace/a"));
        assert_eq!(caps.computer_path(), None);
    }
}
