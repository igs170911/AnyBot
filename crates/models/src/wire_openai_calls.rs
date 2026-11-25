use serde_json::Value;

use crate::types::{ToolArguments, ToolCall};

pub(crate) struct PartialCall {
    pub index: usize,
    pub id: String,
    pub name: String,
    pub arguments: String,
}

impl PartialCall {
    pub(crate) fn finish(self) -> ToolCall {
        let arguments = serde_json::from_str::<Value>(&self.arguments)
            .map(ToolArguments::Json)
            .unwrap_or(ToolArguments::Broken(self.arguments));
        let id = if self.id.is_empty() {
            format!("call_{}", self.index)
        } else {
            self.id
        };
        ToolCall {
            id,
            name: self.name,
            arguments,
        }
    }
}

pub(crate) fn append_calls(calls: &mut Vec<PartialCall>, value: Option<&Value>) {
    let Some(list) = value.and_then(Value::as_array) else {
        return;
    };
    for item in list {
        let index =
            usize::try_from(item.get("index").and_then(Value::as_u64).unwrap_or(0)).unwrap_or(0);
        while calls.len() <= index {
            let next = calls.len();
            calls.push(PartialCall {
                index: next,
                id: String::new(),
                name: String::new(),
                arguments: String::new(),
            });
        }
        let Some(slot) = calls.get_mut(index) else {
            continue;
        };
        fill_slot(slot, item, index);
    }
}

fn fill_slot(slot: &mut PartialCall, item: &Value, index: usize) {
    slot.index = index;
    if let Some(id) = item.get("id").and_then(Value::as_str) {
        crate::jsonutil::set_owned(&mut slot.id, id);
    }
    if let Some(function) = item.get("function") {
        if let Some(name) = function.get("name").and_then(Value::as_str) {
            crate::jsonutil::set_owned(&mut slot.name, name);
        }
        if let Some(arguments) = function.get("arguments").and_then(Value::as_str) {
            slot.arguments.push_str(arguments);
        }
    }
}
