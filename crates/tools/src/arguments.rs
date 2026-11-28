use serde_json::Value;

use crate::registry::{TOOL_EDIT_FILE, TOOL_LIST_DIR, TOOL_READ_FILE, TOOL_SHELL, is_known_tool};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditArgs {
    pub path: String,
    pub old_text: String,
    pub new_text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadArgs {
    pub path: String,
    pub offset: usize,
    pub limit: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListArgs {
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellArgs {
    pub line: String,
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolArgs {
    Edit(EditArgs),
    Read(ReadArgs),
    List(ListArgs),
    Shell(ShellArgs),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArgsRejection {
    UnknownTool {
        tool: String,
        available: Vec<String>,
    },
    NotInTurn {
        tool: String,
        available: Vec<String>,
    },
    OtherComputer,
    Malformed {
        reason: String,
    },
    MissingField {
        field: String,
        tool: String,
    },
}

pub fn check_offered(tool: &str, json: &str, offered: &[&str]) -> Result<(), ArgsRejection> {
    if !offered.contains(&tool) {
        return Err(ArgsRejection::NotInTurn {
            tool: tool.to_owned(),
            available: offered.iter().map(|name| (*name).to_owned()).collect(),
        });
    }
    if let Ok(Value::Object(object)) = serde_json::from_str::<Value>(json)
        && object.contains_key(crate::offer::COMPUTER_PARAM)
    {
        return Err(ArgsRejection::OtherComputer);
    }
    Ok(())
}

#[must_use]
pub fn args_rejection_word(tool: &str, json: &str) -> String {
    match parse_call(tool, json) {
        Ok(_) => "arguments-ok".to_owned(),
        Err(ref rejection) => rejection_word(rejection),
    }
}

#[must_use]
pub fn rejection_word(rejection: &ArgsRejection) -> String {
    match rejection {
        ArgsRejection::UnknownTool { tool, .. } => format!("unknown-tool-{tool}"),
        ArgsRejection::NotInTurn { tool, .. } => format!("not-in-turn-{tool}"),
        ArgsRejection::OtherComputer => "other-computer".to_owned(),
        ArgsRejection::Malformed { reason } => format!("malformed-{reason}"),
        ArgsRejection::MissingField { field, tool } => format!("missing-field-{field}-{tool}"),
    }
}

pub fn parse_call(tool: &str, json: &str) -> Result<ToolArgs, ArgsRejection> {
    if !is_known_tool(tool) {
        return Err(ArgsRejection::UnknownTool {
            tool: tool.to_owned(),
            available: crate::registry::tool_list()
                .iter()
                .map(|name| (*name).to_owned())
                .collect(),
        });
    }
    let parsed: Value = serde_json::from_str(json).map_err(|e| ArgsRejection::Malformed {
        reason: e.to_string(),
    })?;
    let object = parsed.as_object().ok_or_else(|| ArgsRejection::Malformed {
        reason: "the arguments are not a JSON object".to_owned(),
    })?;
    match tool {
        TOOL_EDIT_FILE => parse_edit(object),
        TOOL_READ_FILE => parse_read(object),
        TOOL_LIST_DIR => parse_list(object),
        TOOL_SHELL => parse_shell(object),
        _ => Err(ArgsRejection::Malformed {
            reason: "unknown tool".to_owned(),
        }),
    }
}

fn parse_edit(object: &serde_json::Map<String, Value>) -> Result<ToolArgs, ArgsRejection> {
    let path = text_field(object, "path", TOOL_EDIT_FILE)?;
    let old_text = text_field(object, "old_text", TOOL_EDIT_FILE)?;
    let new_text = text_field(object, "new_text", TOOL_EDIT_FILE)?;
    Ok(ToolArgs::Edit(EditArgs {
        path,
        old_text,
        new_text,
    }))
}

fn parse_read(object: &serde_json::Map<String, Value>) -> Result<ToolArgs, ArgsRejection> {
    let path = text_field(object, "path", TOOL_READ_FILE)?;
    let offset = small_field(object, "offset")?.unwrap_or(0);
    let limit = small_field(object, "limit")?.unwrap_or(crate::read::DEFAULT_READ_LIMIT);
    Ok(ToolArgs::Read(ReadArgs {
        path,
        offset,
        limit,
    }))
}

fn parse_shell(object: &serde_json::Map<String, Value>) -> Result<ToolArgs, ArgsRejection> {
    let line = text_field(object, "command", TOOL_SHELL)?;
    let timeout_ms = number_field(object, "timeout_ms");
    Ok(ToolArgs::Shell(ShellArgs { line, timeout_ms }))
}

fn parse_list(object: &serde_json::Map<String, Value>) -> Result<ToolArgs, ArgsRejection> {
    let path = text_field(object, "path", TOOL_LIST_DIR)?;
    Ok(ToolArgs::List(ListArgs { path }))
}

fn text_field(
    object: &serde_json::Map<String, Value>,
    name: &str,
    tool: &str,
) -> Result<String, ArgsRejection> {
    match object.get(name) {
        Some(Value::String(text)) => Ok(text.clone()),
        Some(_) => Err(ArgsRejection::Malformed {
            reason: format!("{name} is not a string"),
        }),
        None => Err(ArgsRejection::MissingField {
            field: name.to_owned(),
            tool: tool.to_owned(),
        }),
    }
}

fn number_field(object: &serde_json::Map<String, Value>, name: &str) -> Option<u64> {
    object.get(name).and_then(Value::as_u64)
}

fn small_field(
    object: &serde_json::Map<String, Value>,
    name: &str,
) -> Result<Option<usize>, ArgsRejection> {
    match number_field(object, name) {
        Some(value) => usize::try_from(value)
            .map(Some)
            .map_err(|_| ArgsRejection::Malformed {
                reason: format!("{name} does not fit this platform"),
            }),
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arguments_reject_a_tool_that_is_not_in_this_turn() {
        let parsed = parse_call("browser", "{}");
        let Err(ArgsRejection::UnknownTool { tool, available }) = parsed else {
            panic!("unknown tool not rejected: {parsed:?}");
        };
        assert_eq!(tool, "browser");
        assert_eq!(
            available,
            vec![
                "edit_file".to_owned(),
                "read_file".to_owned(),
                "shell".to_owned(),
                "list_dir".to_owned(),
            ]
        );
    }

    #[test]
    fn test_arguments_reject_text_that_is_not_json() {
        let parsed = parse_call(TOOL_SHELL, "not json");
        assert!(matches!(parsed, Err(ArgsRejection::Malformed { .. })));
    }

    #[test]
    fn test_arguments_require_every_field_of_a_call() {
        let parsed = parse_call(TOOL_EDIT_FILE, r#"{"path":"a.txt","old_text":"x"}"#);
        let Err(ArgsRejection::MissingField { field, tool }) = parsed else {
            panic!("missing field not rejected: {parsed:?}");
        };
        assert_eq!(field, "new_text");
        assert_eq!(tool, "edit_file");
    }

    #[test]
    fn test_arguments_read_optional_numbers_with_defaults() {
        let parsed = parse_call(TOOL_READ_FILE, r#"{"path":"a.txt"}"#);
        let Ok(ToolArgs::Read(args)) = parsed else {
            panic!("read args missing: {parsed:?}");
        };
        assert_eq!(args.offset, 0);
        assert_eq!(args.limit, 2000);
    }

    #[test]
    fn shell_rejects_old_line_parameter() {
        let parsed = parse_call(TOOL_SHELL, r#"{"line":"ls"}"#);
        let Err(ArgsRejection::MissingField { field, tool }) = parsed else {
            panic!("old line param not rejected: {parsed:?}");
        };
        assert_eq!(field, "command");
        assert_eq!(tool, "shell");
    }

    #[test]
    fn check_offered_rejects_unoffered_and_computer_param() {
        let offered = [TOOL_READ_FILE, TOOL_SHELL];
        let missing = check_offered(TOOL_EDIT_FILE, "{}", &offered);
        assert_eq!(
            missing,
            Err(ArgsRejection::NotInTurn {
                tool: TOOL_EDIT_FILE.to_owned(),
                available: vec![TOOL_READ_FILE.to_owned(), TOOL_SHELL.to_owned()],
            })
        );
        let routed = check_offered(
            TOOL_SHELL,
            r#"{"command":"ls","computer":"Linux"}"#,
            &offered,
        );
        assert_eq!(routed, Err(ArgsRejection::OtherComputer));
        assert_eq!(
            check_offered(TOOL_SHELL, r#"{"command":"ls"}"#, &offered),
            Ok(())
        );
    }
}
