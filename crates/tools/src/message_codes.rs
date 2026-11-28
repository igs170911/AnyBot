pub const CODE_TOOL_EDIT_CREATED: &str = "tool-edit-created";
pub const CODE_TOOL_EDIT_OVERWRITTEN: &str = "tool-edit-overwritten";
pub const CODE_TOOL_EDIT_MODIFIED: &str = "tool-edit-modified";
pub const CODE_TOOL_EDIT_NOT_FOUND: &str = "tool-edit-not-found";
pub const CODE_TOOL_EDIT_MULTIPLE: &str = "tool-edit-multiple";
pub const CODE_TOOL_EDIT_FILE_MISSING: &str = "tool-edit-file-missing";
pub const CODE_TOOL_READ_TOO_BIG: &str = "tool-read-too-big";
pub const CODE_TOOL_READ_NOT_TEXT: &str = "tool-read-not-text";
pub const CODE_TOOL_READ_DIRECTORY: &str = "tool-read-directory";
pub const CODE_TOOL_READ_LINES: &str = "tool-read-lines";
pub const CODE_TOOL_READ_MORE: &str = "tool-read-more";
pub const CODE_TOOL_LIST_MORE: &str = "tool-list-more";
pub const CODE_TOOL_LIST_EMPTY: &str = "tool-list-empty";
pub const CODE_TOOL_SEARCH_CUT: &str = "tool-search-cut";
pub const CODE_TOOL_SEARCH_EMPTY: &str = "tool-search-empty";
pub const CODE_TOOL_SEARCH_TIMED_OUT: &str = "tool-search-timed-out";
pub const CODE_TOOL_READ_RESULT_WINDOW: &str = "tool-read-result-window";
pub const CODE_TOOL_READ_RESULT_HIT: &str = "tool-read-result-hit";
pub const CODE_TOOL_READ_RESULT_NOT_FOUND: &str = "tool-read-result-not-found";
pub const CODE_TOOL_READ_RESULT_MISSING: &str = "tool-read-result-missing";
pub const CODE_TOOL_READ_RESULT_CUT: &str = "tool-read-result-cut";
pub const CODE_TOOL_SHELL_NO_OUTPUT: &str = "tool-shell-no-output";
pub const CODE_TOOL_SHELL_FAILED: &str = "tool-shell-failed";
pub const CODE_TOOL_SHELL_TRIMMED: &str = "tool-shell-trimmed";
pub const CODE_TOOL_SHELL_TIMED_OUT: &str = "tool-shell-timed-out";
pub const CODE_TOOL_NOT_IN_TURN: &str = "tool-not-in-turn";
pub const CODE_TOOL_OTHER_COMPUTER: &str = "tool-other-computer";
pub const CODE_TOOL_DESC_EDIT_FILE: &str = "tool-desc-edit-file";
pub const CODE_TOOL_DESC_READ_FILE: &str = "tool-desc-read-file";
pub const CODE_TOOL_DESC_SHELL: &str = "tool-desc-shell";
pub const CODE_TOOL_DESC_LIST_DIR: &str = "tool-desc-list-dir";
pub const CODE_TOOL_DESC_SEARCH: &str = "tool-desc-search";
pub const CODE_TOOL_DESC_READ_RESULT: &str = "tool-desc-read-result";
pub const CODE_TOOL_DESC_ASK_PERSON: &str = "tool-desc-ask-person";
pub const CODE_TOOL_ASK_TOO_MANY: &str = "tool-ask-too-many";
pub const CODE_QUESTION_SKIPPED: &str = "question-skipped";
pub const CODE_QUESTION_UNANSWERED: &str = "question-unanswered";

#[must_use]
pub fn all_tool_codes() -> [&'static str; 37] {
    [
        CODE_TOOL_EDIT_CREATED,
        CODE_TOOL_EDIT_OVERWRITTEN,
        CODE_TOOL_EDIT_MODIFIED,
        CODE_TOOL_EDIT_NOT_FOUND,
        CODE_TOOL_EDIT_MULTIPLE,
        CODE_TOOL_EDIT_FILE_MISSING,
        CODE_TOOL_READ_TOO_BIG,
        CODE_TOOL_READ_NOT_TEXT,
        CODE_TOOL_READ_DIRECTORY,
        CODE_TOOL_READ_LINES,
        CODE_TOOL_READ_MORE,
        CODE_TOOL_LIST_MORE,
        CODE_TOOL_LIST_EMPTY,
        CODE_TOOL_SEARCH_CUT,
        CODE_TOOL_SEARCH_EMPTY,
        CODE_TOOL_SEARCH_TIMED_OUT,
        CODE_TOOL_READ_RESULT_WINDOW,
        CODE_TOOL_READ_RESULT_HIT,
        CODE_TOOL_READ_RESULT_NOT_FOUND,
        CODE_TOOL_READ_RESULT_MISSING,
        CODE_TOOL_READ_RESULT_CUT,
        CODE_TOOL_SHELL_NO_OUTPUT,
        CODE_TOOL_SHELL_FAILED,
        CODE_TOOL_SHELL_TRIMMED,
        CODE_TOOL_SHELL_TIMED_OUT,
        CODE_TOOL_NOT_IN_TURN,
        CODE_TOOL_OTHER_COMPUTER,
        CODE_TOOL_DESC_EDIT_FILE,
        CODE_TOOL_DESC_READ_FILE,
        CODE_TOOL_DESC_SHELL,
        CODE_TOOL_DESC_LIST_DIR,
        CODE_TOOL_DESC_SEARCH,
        CODE_TOOL_DESC_READ_RESULT,
        CODE_TOOL_DESC_ASK_PERSON,
        CODE_TOOL_ASK_TOO_MANY,
        CODE_QUESTION_SKIPPED,
        CODE_QUESTION_UNANSWERED,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_message_codes_are_unique() {
        let mut seen: Vec<&str> = Vec::new();
        for code in all_tool_codes() {
            assert!(!seen.contains(&code), "duplicate code {code}");
            seen.push(code);
        }
        assert_eq!(seen.len(), all_tool_codes().len());
    }
}
