use anybot_protocol::ComputerCaps;
use serde_json::{Value, json};

use crate::message_codes::{
    CODE_TOOL_DESC_EDIT_FILE, CODE_TOOL_DESC_LIST_DIR, CODE_TOOL_DESC_READ_FILE,
    CODE_TOOL_DESC_READ_RESULT, CODE_TOOL_DESC_SEARCH, CODE_TOOL_DESC_SHELL,
};
use crate::registry::{
    TOOL_EDIT_FILE, TOOL_LIST_DIR, TOOL_READ_FILE, TOOL_READ_RESULT, TOOL_SEARCH, TOOL_SHELL,
    registry,
};

pub const COMPUTER_PARAM: &str = "computer";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolDef {
    pub name: &'static str,
    pub description_code: &'static str,
    pub parameters: Value,
}

#[must_use]
pub fn offered_tools(caps: &ComputerCaps) -> Vec<ToolDef> {
    let mut out = Vec::new();
    for (name, _) in registry() {
        if let Some(def) = tool_if_offered(name, caps) {
            out.push(def);
        }
    }
    out
}

fn tool_if_offered(name: &str, caps: &ComputerCaps) -> Option<ToolDef> {
    match name {
        TOOL_EDIT_FILE if caps.write_files.yes() => Some(edit_def()),
        TOOL_READ_FILE if caps.read_files.yes() => Some(read_def()),
        TOOL_SHELL if caps.run_commands.yes() => Some(shell_def()),
        TOOL_LIST_DIR if caps.read_files.yes() => Some(list_def()),
        TOOL_SEARCH if caps.read_files.yes() => Some(search_def()),
        TOOL_READ_RESULT => Some(read_result_def()),
        _ => None,
    }
}

fn edit_def() -> ToolDef {
    ToolDef {
        name: TOOL_EDIT_FILE,
        description_code: CODE_TOOL_DESC_EDIT_FILE,
        parameters: json!({
            "type": "object",
            "properties": {
                "path": { "type": "string" },
                "old_text": { "type": "string" },
                "new_text": { "type": "string" }
            },
            "required": ["path", "old_text", "new_text"]
        }),
    }
}

fn read_def() -> ToolDef {
    ToolDef {
        name: TOOL_READ_FILE,
        description_code: CODE_TOOL_DESC_READ_FILE,
        parameters: json!({
            "type": "object",
            "properties": {
                "path": { "type": "string" },
                "offset": { "type": "integer", "minimum": 0 },
                "limit": { "type": "integer", "minimum": 1 }
            },
            "required": ["path"]
        }),
    }
}

fn shell_def() -> ToolDef {
    ToolDef {
        name: TOOL_SHELL,
        description_code: CODE_TOOL_DESC_SHELL,
        parameters: json!({
            "type": "object",
            "properties": {
                "command": { "type": "string" },
                "timeout_ms": { "type": "integer", "minimum": 1 }
            },
            "required": ["command"]
        }),
    }
}

fn list_def() -> ToolDef {
    ToolDef {
        name: TOOL_LIST_DIR,
        description_code: CODE_TOOL_DESC_LIST_DIR,
        parameters: json!({
            "type": "object",
            "properties": {
                "path": { "type": "string" }
            },
            "required": ["path"]
        }),
    }
}

fn search_def() -> ToolDef {
    ToolDef {
        name: TOOL_SEARCH,
        description_code: CODE_TOOL_DESC_SEARCH,
        parameters: json!({
            "type": "object",
            "properties": {
                "pattern": { "type": "string" },
                "path": { "type": "string" },
                "files_only": { "type": "boolean" }
            },
            "required": ["pattern"]
        }),
    }
}

fn read_result_def() -> ToolDef {
    ToolDef {
        name: TOOL_READ_RESULT,
        description_code: CODE_TOOL_DESC_READ_RESULT,
        parameters: json!({
            "type": "object",
            "properties": {
                "id": { "type": "string" },
                "from": { "type": "integer", "minimum": 0 },
                "chars": { "type": "integer", "minimum": 1 },
                "find": { "type": "string" }
            },
            "required": ["id"]
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anybot_protocol::{CapState, ComputerOwner, ComputerPlatform};

    fn names_of(defs: &[ToolDef]) -> Vec<&'static str> {
        defs.iter().map(|def| def.name).collect()
    }

    #[test]
    fn offered_tools_follow_each_capability() {
        let base = ComputerCaps::declared(ComputerPlatform::Linux, ComputerOwner::User);
        assert_eq!(names_of(&offered_tools(&base)), vec![TOOL_READ_RESULT]);

        let only_read = ComputerCaps {
            read_files: CapState::Supported,
            ..base.clone()
        };
        assert_eq!(
            names_of(&offered_tools(&only_read)),
            vec![TOOL_READ_FILE, TOOL_LIST_DIR, TOOL_SEARCH, TOOL_READ_RESULT]
        );

        let only_write = ComputerCaps {
            write_files: CapState::Supported,
            ..base.clone()
        };
        assert_eq!(
            names_of(&offered_tools(&only_write)),
            vec![TOOL_EDIT_FILE, TOOL_READ_RESULT]
        );

        let only_shell = ComputerCaps {
            run_commands: CapState::Supported,
            ..base
        };
        assert_eq!(
            names_of(&offered_tools(&only_shell)),
            vec![TOOL_SHELL, TOOL_READ_RESULT]
        );
    }
}
