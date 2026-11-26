#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolLevel {
    ReadOnly,
    Irreversible,
}

pub const TOOL_EDIT_FILE: &str = "edit_file";
pub const TOOL_READ_FILE: &str = "read_file";
pub const TOOL_SHELL: &str = "shell";

#[must_use]
pub fn tool_list() -> [&'static str; 3] {
    [TOOL_EDIT_FILE, TOOL_READ_FILE, TOOL_SHELL]
}

#[must_use]
pub fn level_of(tool: &str) -> ToolLevel {
    match tool {
        TOOL_READ_FILE => ToolLevel::ReadOnly,
        _ => ToolLevel::Irreversible,
    }
}

#[must_use]
pub fn registry() -> [(&'static str, ToolLevel); 3] {
    [
        (TOOL_EDIT_FILE, level_of(TOOL_EDIT_FILE)),
        (TOOL_READ_FILE, level_of(TOOL_READ_FILE)),
        (TOOL_SHELL, level_of(TOOL_SHELL)),
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
        assert_eq!(table.len(), 3);
        assert_eq!(level_of(TOOL_READ_FILE), ToolLevel::ReadOnly);
        assert_eq!(level_of(TOOL_EDIT_FILE), ToolLevel::Irreversible);
        assert_eq!(level_of(TOOL_SHELL), ToolLevel::Irreversible);
    }

    #[test]
    fn test_registry_marks_unknown_tools_as_irreversible() {
        assert_eq!(level_of("list_dir"), ToolLevel::Irreversible);
        assert!(!is_known_tool("list_dir"));
        assert!(is_known_tool(TOOL_SHELL));
    }
}
