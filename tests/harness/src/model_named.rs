use std::collections::HashMap;

use crate::model_script::{Script, Step};

#[derive(Default)]
pub struct NamedScripts {
    queues: HashMap<String, Script>,
}

impl NamedScripts {
    pub fn set(&mut self, model: &str, steps: Vec<Step>) {
        let mut script = Script::default();
        for step in steps {
            script.push(step);
        }
        self.queues.insert(model.to_owned(), script);
    }

    pub fn take(&mut self, model: &str) -> Option<Option<Step>> {
        self.queues.get_mut(model).map(Script::take)
    }

    #[must_use]
    pub fn len(&self, model: &str) -> usize {
        self.queues.get(model).map_or(0, Script::len)
    }
}

#[must_use]
pub fn wants_stream(body: &str) -> bool {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(body) else {
        return false;
    };
    value
        .get("stream")
        .is_some_and(|s| s.as_bool().unwrap_or(false))
}

#[must_use]
pub fn requested_model(body: &str) -> String {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(body) else {
        return String::new();
    };
    value
        .get("model")
        .and_then(|m| m.as_str())
        .unwrap_or_default()
        .to_owned()
}
