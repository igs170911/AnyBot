use anybot_trust::{EditKind, EditRejection};

use crate::message_codes::{
    CODE_TOOL_EDIT_CREATED, CODE_TOOL_EDIT_FILE_MISSING, CODE_TOOL_EDIT_MODIFIED,
    CODE_TOOL_EDIT_MULTIPLE, CODE_TOOL_EDIT_NOT_FOUND, CODE_TOOL_EDIT_OVERWRITTEN,
};

#[must_use]
pub fn edit_code(kind: EditKind) -> &'static str {
    match kind {
        EditKind::Created => CODE_TOOL_EDIT_CREATED,
        EditKind::Overwritten => CODE_TOOL_EDIT_OVERWRITTEN,
        EditKind::Modified => CODE_TOOL_EDIT_MODIFIED,
    }
}

#[must_use]
pub fn rejection_code(rejection: &EditRejection) -> &'static str {
    match rejection {
        EditRejection::NotFound => CODE_TOOL_EDIT_NOT_FOUND,
        EditRejection::Multiple { .. } => CODE_TOOL_EDIT_MULTIPLE,
        EditRejection::FileMissing => CODE_TOOL_EDIT_FILE_MISSING,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_edit_result_codes_name_each_kind() {
        assert_eq!(edit_code(EditKind::Created), "tool-edit-created");
        assert_eq!(edit_code(EditKind::Overwritten), "tool-edit-overwritten");
        assert_eq!(edit_code(EditKind::Modified), "tool-edit-modified");
    }

    #[test]
    fn test_edit_rejection_codes_name_each_rule() {
        assert_eq!(
            rejection_code(&EditRejection::NotFound),
            "tool-edit-not-found"
        );
        assert_eq!(
            rejection_code(&EditRejection::Multiple { hits: 3 }),
            "tool-edit-multiple",
        );
        assert_eq!(
            rejection_code(&EditRejection::FileMissing),
            "tool-edit-file-missing",
        );
    }
}
