#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolLevel {
    ReadOnly,
    Reversible,
    Irreversible,
}

pub const TOOL_ASK_PERSON: &str = "ask_person";
pub const TOOL_EDIT_FILE: &str = "edit_file";
pub const TOOL_READ_FILE: &str = "read_file";
pub const TOOL_SHELL: &str = "shell";
pub const TOOL_LIST_DIR: &str = "list_dir";
pub const TOOL_SEARCH: &str = "search";
pub const TOOL_READ_RESULT: &str = "read_result";

#[must_use]
pub fn tool_list() -> [&'static str; 7] {
    [
        TOOL_EDIT_FILE,
        TOOL_READ_FILE,
        TOOL_SHELL,
        TOOL_LIST_DIR,
        TOOL_SEARCH,
        TOOL_READ_RESULT,
        TOOL_ASK_PERSON,
    ]
}

#[must_use]
pub fn level_of(tool: &str) -> ToolLevel {
    if !is_known_tool(tool) {
        return ToolLevel::Irreversible;
    }
    match tool {
        TOOL_READ_FILE | TOOL_LIST_DIR | TOOL_SEARCH | TOOL_READ_RESULT | TOOL_ASK_PERSON => {
            ToolLevel::ReadOnly
        }
        TOOL_EDIT_FILE => ToolLevel::Reversible,
        _ => ToolLevel::Irreversible,
    }
}

#[must_use]
pub fn registry() -> [(&'static str, ToolLevel); 7] {
    [
        (TOOL_EDIT_FILE, level_of(TOOL_EDIT_FILE)),
        (TOOL_READ_FILE, level_of(TOOL_READ_FILE)),
        (TOOL_SHELL, level_of(TOOL_SHELL)),
        (TOOL_LIST_DIR, level_of(TOOL_LIST_DIR)),
        (TOOL_SEARCH, level_of(TOOL_SEARCH)),
        (TOOL_READ_RESULT, level_of(TOOL_READ_RESULT)),
        (TOOL_ASK_PERSON, level_of(TOOL_ASK_PERSON)),
    ]
}

#[must_use]
pub fn is_known_tool(tool: &str) -> bool {
    tool_list().contains(&tool)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_lists_each_tool_with_exactly_one_level() {
        let table = registry();
        assert_eq!(table.len(), 7);
        assert_eq!(level_of(TOOL_READ_FILE), ToolLevel::ReadOnly);
        assert_eq!(level_of(TOOL_LIST_DIR), ToolLevel::ReadOnly);
        assert_eq!(level_of(TOOL_SEARCH), ToolLevel::ReadOnly);
        assert_eq!(level_of(TOOL_READ_RESULT), ToolLevel::ReadOnly);
        assert_eq!(level_of(TOOL_ASK_PERSON), ToolLevel::ReadOnly);
        assert_eq!(level_of(TOOL_EDIT_FILE), ToolLevel::Reversible);
        assert_eq!(level_of(TOOL_SHELL), ToolLevel::Irreversible);
    }

    #[test]
    fn test_registry_marks_unknown_tools_as_irreversible() {
        assert_eq!(level_of("browser"), ToolLevel::Irreversible);
        assert!(!is_known_tool("browser"));
        assert!(is_known_tool(TOOL_SHELL));
        assert!(is_known_tool(TOOL_LIST_DIR));
        assert!(is_known_tool(TOOL_SEARCH));
        assert!(is_known_tool(TOOL_READ_RESULT));
    }
}
