use anybot_protocol::{QuestionItem, QuestionKind};
use serde_json::{Map, Value};

use crate::arguments::ArgsRejection;
use crate::message_codes::CODE_TOOL_ASK_TOO_MANY;

pub const MAX_QUESTIONS: usize = 6;
pub const MAX_OPTIONS: usize = 8;
pub const MAX_TITLE_CHARS: usize = 300;
pub const MAX_OPTION_CHARS: usize = 80;
pub const MAX_PREFILL_CHARS: usize = 500;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AskPersonArgs {
    pub questions: Vec<QuestionItem>,
}

pub fn parse_ask_person(object: &Map<String, Value>) -> Result<AskPersonArgs, ArgsRejection> {
    let raw = object
        .get("questions")
        .ok_or_else(|| ArgsRejection::MissingField {
            field: "questions".to_owned(),
            tool: crate::registry::TOOL_ASK_PERSON.to_owned(),
        })?;
    let list = raw.as_array().ok_or_else(|| ArgsRejection::Malformed {
        reason: "questions is not an array".to_owned(),
    })?;
    if list.is_empty() {
        return Err(ArgsRejection::Malformed {
            reason: "questions must not be empty".to_owned(),
        });
    }
    if list.len() > MAX_QUESTIONS {
        return Err(ArgsRejection::TooMany {
            code: CODE_TOOL_ASK_TOO_MANY,
        });
    }
    let mut questions = Vec::with_capacity(list.len());
    for entry in list {
        questions.push(parse_item(entry)?);
    }
    Ok(AskPersonArgs { questions })
}

fn parse_item(value: &Value) -> Result<QuestionItem, ArgsRejection> {
    let obj = value.as_object().ok_or_else(|| ArgsRejection::Malformed {
        reason: "question entry is not an object".to_owned(),
    })?;
    let raw_title = item_title(obj)?;
    if raw_title.trim().is_empty() {
        return Err(ArgsRejection::Malformed {
            reason: "question title must not be empty".to_owned(),
        });
    }
    let options = item_options(obj)?;
    if options.len() > MAX_OPTIONS {
        return Err(ArgsRejection::TooMany {
            code: CODE_TOOL_ASK_TOO_MANY,
        });
    }
    let kind = item_kind(obj, &options)?;
    let prefill = item_prefill(obj)?;
    Ok(QuestionItem {
        title: truncate_chars(&raw_title, MAX_TITLE_CHARS),
        kind,
        options,
        prefill,
    })
}

fn item_title(obj: &Map<String, Value>) -> Result<String, ArgsRejection> {
    if let Some(val) = obj.get("title").or_else(|| obj.get("question")) {
        return val
            .as_str()
            .map(ToOwned::to_owned)
            .ok_or_else(|| ArgsRejection::Malformed {
                reason: "title is not a string".to_owned(),
            });
    }
    Err(ArgsRejection::MissingField {
        field: "title".to_owned(),
        tool: crate::registry::TOOL_ASK_PERSON.to_owned(),
    })
}

fn item_options(obj: &Map<String, Value>) -> Result<Vec<String>, ArgsRejection> {
    let Some(raw) = obj.get("options") else {
        return Ok(Vec::new());
    };
    let arr = raw.as_array().ok_or_else(|| ArgsRejection::Malformed {
        reason: "options is not an array".to_owned(),
    })?;
    let mut out = Vec::with_capacity(arr.len());
    for item in arr {
        let s = item.as_str().ok_or_else(|| ArgsRejection::Malformed {
            reason: "option is not a string".to_owned(),
        })?;
        out.push(truncate_chars(s, MAX_OPTION_CHARS));
    }
    Ok(out)
}

fn item_kind(obj: &Map<String, Value>, options: &[String]) -> Result<QuestionKind, ArgsRejection> {
    if let Some(raw) = obj.get("kind") {
        let s = raw.as_str().ok_or_else(|| ArgsRejection::Malformed {
            reason: "kind is not a string".to_owned(),
        })?;
        return match s {
            "single" => Ok(QuestionKind::Single),
            "multi" => Ok(QuestionKind::Multi),
            "text" => Ok(QuestionKind::Text),
            _ => Err(ArgsRejection::Malformed {
                reason: format!("unknown question kind {s}"),
            }),
        };
    }
    if obj.get("multiple").and_then(Value::as_bool) == Some(true) {
        return Ok(QuestionKind::Multi);
    }
    if options.is_empty() {
        Ok(QuestionKind::Text)
    } else {
        Ok(QuestionKind::Single)
    }
}

fn item_prefill(obj: &Map<String, Value>) -> Result<Option<String>, ArgsRejection> {
    let Some(raw) = obj.get("prefill").or_else(|| obj.get("default")) else {
        return Ok(None);
    };
    if raw.is_null() {
        return Ok(None);
    }
    let text = raw.as_str().ok_or_else(|| ArgsRejection::Malformed {
        reason: "prefill is not a string".to_owned(),
    })?;
    Ok(Some(truncate_chars(text, MAX_PREFILL_CHARS)))
}

#[must_use]
pub fn truncate_chars(text: &str, max_chars: usize) -> String {
    text.chars().take(max_chars).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arguments::{args_rejection_word, parse_call};
    use crate::registry::TOOL_ASK_PERSON;

    #[test]
    fn ask_person_args_validation() {
        let too_many_q = serde_json::json!({
            "questions": (0..7).map(|i| serde_json::json!({"title": format!("Q{i}"), "kind": "text"})).collect::<Vec<_>>()
        })
        .to_string();
        assert_eq!(
            args_rejection_word(TOOL_ASK_PERSON, &too_many_q),
            CODE_TOOL_ASK_TOO_MANY
        );

        let too_many_opt = serde_json::json!({
            "questions": [{
                "title": "Pick",
                "kind": "single",
                "options": (0..9).map(|i| format!("o{i}")).collect::<Vec<_>>()
            }]
        })
        .to_string();
        assert_eq!(
            args_rejection_word(TOOL_ASK_PERSON, &too_many_opt),
            CODE_TOOL_ASK_TOO_MANY
        );

        let long_title = "a".repeat(301);
        let long_opt = "b".repeat(81);
        let long_pre = "c".repeat(501);
        let truncated_json = serde_json::json!({
            "questions": [{
                "title": long_title,
                "kind": "multi",
                "options": [long_opt],
                "prefill": long_pre
            }]
        })
        .to_string();
        let Ok(crate::arguments::ToolArgs::AskPerson(parsed)) =
            parse_call(TOOL_ASK_PERSON, &truncated_json)
        else {
            panic!("expected AskPerson args");
        };
        let item = parsed.questions.first().unwrap();
        assert_eq!(item.title.chars().count(), 300);
        assert_eq!(item.options.first().unwrap().chars().count(), 80);
        assert_eq!(item.prefill.as_ref().unwrap().chars().count(), 500);
        assert_eq!(item.kind, QuestionKind::Multi);
    }
}
