use std::path::Path;

use anybot_protocol::{CapState, ComputerFailure, TargetKind};
use anybot_trust::{EditRejection, OpRejection};

pub const READ_LIMIT_BYTES: usize = 8_000_000;

pub(crate) fn cap_pair(on: bool) -> CapState {
    if on {
        CapState::Supported
    } else {
        CapState::Unsupported
    }
}

pub(crate) fn target_kind_of(path: &Path) -> Option<TargetKind> {
    if path.is_dir() {
        return Some(TargetKind::Directory);
    }
    if path.exists() {
        return Some(TargetKind::File);
    }
    None
}

pub(crate) fn read_bytes(full: &Path) -> Result<Vec<u8>, OpRejection> {
    std::fs::read(full).map_err(|e| {
        OpRejection::from_failure(ComputerFailure::operation(format!("cannot read: {e}")))
    })
}

pub(crate) fn text_of(bytes: Vec<u8>) -> Result<String, OpRejection> {
    if bytes.len() > READ_LIMIT_BYTES {
        return Err(OpRejection::from_failure(ComputerFailure::operation(
            "the file is larger than the read limit".to_owned(),
        )));
    }
    String::from_utf8(bytes).map_err(|_| {
        OpRejection::from_failure(ComputerFailure::operation(
            "the file is not text".to_owned(),
        ))
    })
}

pub(crate) fn write_content(full: &Path, content: &str) -> Result<(), OpRejection> {
    if let Some(parent) = full.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            OpRejection::from_failure(ComputerFailure::operation(format!(
                "cannot prepare the folder: {e}"
            )))
        })?;
    }
    std::fs::write(full, content).map_err(|e| {
        OpRejection::from_failure(ComputerFailure::operation(format!("cannot write: {e}")))
    })
}

pub(crate) fn size_u64(len: usize) -> Result<u64, OpRejection> {
    u64::try_from(len).map_err(|_| {
        OpRejection::from_failure(ComputerFailure::operation(
            "the size does not fit this platform".to_owned(),
        ))
    })
}

pub(crate) fn edit_failure(rejection: &EditRejection) -> OpRejection {
    match rejection {
        EditRejection::NotFound => OpRejection::from_failure(ComputerFailure::operation(
            "the text to replace was not found in the file".to_owned(),
        )),
        EditRejection::Multiple { hits } => OpRejection::from_failure(ComputerFailure::operation(
            format!("the text to replace appears {hits} times in the file"),
        )),
        EditRejection::FileMissing => OpRejection::from_failure(ComputerFailure::operation(
            "the file does not exist".to_owned(),
        )),
    }
}
