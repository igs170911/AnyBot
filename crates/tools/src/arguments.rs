use serde_json::Value;

use crate::ask_person::{AskPersonArgs, parse_ask_person};
use crate::registry::{TOOL_ASK_PERSON, TOOL_EDIT_FILE, TOOL_READ_FILE, TOOL_SHELL, is_known_tool};

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
pub struct ShellArgs {
    pub line: String,
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolArgs {
    Edit(EditArgs),
    Read(ReadArgs),
    Shell(ShellArgs),
    AskPerson(AskPersonArgs),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArgsRejection {
    UnknownTool {
        tool: String,
        available: Vec<String>,
    },
    Malformed {
        reason: String,
    },
    MissingField {
        field: String,
        tool: String,
    },
    TooMany {
        code: &'static str,
    },
}

#[must_use]
pub fn args_rejection_word(tool: &str, json: &str) -> String {
    match parse_call(tool, json) {
        Ok(_) => "arguments-ok".to_owned(),
        Err(ArgsRejection::UnknownTool { tool, .. }) => format!("unknown-tool-{tool}"),
        Err(ArgsRejection::Malformed { reason }) => format!("malformed-{reason}"),
        Err(ArgsRejection::MissingField { field, tool }) => format!("missing-field-{field}-{tool}"),
        Err(ArgsRejection::TooMany { code }) => code.to_owned(),
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
        TOOL_SHELL => parse_shell(object),
        TOOL_ASK_PERSON => parse_ask_person(object).map(ToolArgs::AskPerson),
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
    let limit = small_field(object, "limit")?.unwrap_or(2000);
    Ok(ToolArgs::Read(ReadArgs {
        path,
        offset,
        limit,
    }))
}

fn parse_shell(object: &serde_json::Map<String, Value>) -> Result<ToolArgs, ArgsRejection> {
    let line = text_field(object, "line", TOOL_SHELL)?;
    let timeout_ms = number_field(object, "timeout_ms");
    Ok(ToolArgs::Shell(ShellArgs { line, timeout_ms }))
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
        let parsed = parse_call("list_dir", "{}");
        let Err(ArgsRejection::UnknownTool { tool, available }) = parsed else {
            panic!("unknown tool not rejected: {parsed:?}");
        };
        assert_eq!(tool, "list_dir");
        assert_eq!(
            available,
            vec![
                "edit_file".to_owned(),
                "read_file".to_owned(),
                "shell".to_owned(),
                "ask_person".to_owned()
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
}
