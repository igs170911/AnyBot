use anybot_messages::Lang;
use anybot_protocol::{ActualResult, GateRecord};
use anybot_tools::read::DEFAULT_READ_LIMIT;
use anybot_trust::{ModelResultView, OpRejection, OutcomeText};

use crate::error::TurnError;
pub(crate) use crate::view_text::render_text;
use crate::view_text::{command_text, list_text, read_text, write_text};

#[derive(Debug)]
pub(crate) struct ActionView {
    lang: Lang,
    edit_lines: u32,
    read_offset: usize,
    read_limit: usize,
    read_path: String,
    computer: String,
    edit_path: String,
    output: std::sync::Mutex<Option<String>>,
    body: std::sync::Mutex<Option<String>>,
}

impl ActionView {
    #[must_use]
    pub(crate) fn new(lang: Lang) -> Self {
        ActionView {
            lang,
            edit_lines: 0,
            read_offset: 0,
            read_limit: DEFAULT_READ_LIMIT,
            read_path: String::new(),
            computer: String::new(),
            edit_path: String::new(),
            output: std::sync::Mutex::new(None),
            body: std::sync::Mutex::new(None),
        }
    }

    #[must_use]
    pub(crate) fn with_edit_lines(mut self, lines: u32) -> Self {
        self.edit_lines = lines;
        self
    }

    #[must_use]
    pub(crate) fn with_edit_path(mut self, path: &str) -> Self {
        path.clone_into(&mut self.edit_path);
        self
    }

    pub(crate) fn with_computer(mut self, computer: &str) -> Self {
        computer.clone_into(&mut self.computer);
        self
    }

    #[must_use]
    pub(crate) fn computer_name(&self) -> &str {
        &self.computer
    }

    #[must_use]
    pub(crate) fn with_read_window(mut self, offset: usize, limit: usize) -> Self {
        self.read_offset = offset;
        self.read_limit = limit;
        self
    }

    #[must_use]
    pub(crate) fn with_read_path(mut self, path: &str) -> Self {
        path.clone_into(&mut self.read_path);
        self
    }

    pub(crate) fn render(&self, text: &OutcomeText) -> Result<String, TurnError> {
        render_text(self.lang, text)
    }
}

impl ModelResultView for ActionView {
    fn result_text(&self, _tool: &str, result: &ActualResult) -> OutcomeText {
        match result {
            ActualResult::Command(outcome) => {
                self.capture(&outcome.output);
                command_text(outcome.exit_code)
            }
            ActualResult::Write(outcome) => write_text(outcome, self.edit_lines, &self.edit_path),
            ActualResult::Read(outcome) => {
                let (header, body) = read_text(
                    self.lang,
                    outcome,
                    self.read_offset,
                    self.read_limit,
                    &self.read_path,
                );
                if let Some(text) = body {
                    self.capture_body(&text);
                }
                header
            }
            ActualResult::List(outcome) => {
                let (header, body) = list_text(self.lang, outcome);
                if let Some(text) = body {
                    self.capture_body(&text);
                }
                header
            }
            ActualResult::Path(_) | ActualResult::Caps(_) => {
                OutcomeText::of("tool-empty", Vec::new())
            }
        }
    }

    fn failure_text(&self, rejection: &OpRejection) -> OutcomeText {
        if rejection.reason == anybot_trust::EDIT_NOT_FOUND_REASON {
            return OutcomeText::of(
                "tool-edit-not-found",
                vec![("path".to_owned(), self.edit_path.clone())],
            );
        }
        if let Some(count) = rejection
            .reason
            .strip_prefix(anybot_trust::EDIT_MULTIPLE_REASON)
        {
            let hits = count.trim().to_owned();
            return OutcomeText::of(
                "tool-edit-multiple",
                vec![
                    ("path".to_owned(), self.edit_path.clone()),
                    ("count".to_owned(), hits),
                ],
            );
        }
        OutcomeText::of(
            "tool-operation-failed",
            vec![("reason".to_owned(), rejection.reason.clone())],
        )
    }

    fn rejection_text(&self, gate: &GateRecord, params: &[(String, String)]) -> OutcomeText {
        if let Some(code) = floor_rule_code(&gate.rule) {
            let wanted = floor_params(code);
            let kept: Vec<(String, String)> = params
                .iter()
                .filter(|(k, _)| wanted.contains(&k.as_str()))
                .cloned()
                .collect();
            return OutcomeText::of(code, kept);
        }
        let mut merged: std::collections::BTreeMap<String, String> =
            params.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
        merged
            .entry("computer".to_owned())
            .or_insert_with(|| self.computer.clone());
        merged.insert("rule".to_owned(), gate.rule.clone());
        merged.insert("refusal".to_owned(), String::new());
        let all: Vec<(String, String)> = merged.into_iter().collect();
        OutcomeText::of("gate-denied", all)
    }
}

fn floor_params(code: &str) -> Vec<&'static str> {
    match code {
        "gate-command-denied" => vec!["command"],
        "gate-private-file" | "gate-bot-machine-outside" => vec!["path"],
        _ => Vec::new(),
    }
}

fn floor_rule_code(rule: &str) -> Option<&'static str> {
    match rule {
        "sandbox-deny" => Some("gate-command-denied"),
        "private-file" => Some("gate-private-file"),
        "bot-machine-outside" => Some("gate-bot-machine-outside"),
        _ => None,
    }
}

impl ActionView {
    fn capture(&self, output: &str) {
        if let Ok(mut guard) = self.output.lock() {
            *guard = Some(output.to_owned());
        }
    }

    fn capture_body(&self, body: &str) {
        if let Ok(mut guard) = self.body.lock() {
            *guard = Some(body.to_owned());
        }
    }

    #[must_use]
    pub(crate) fn take_output(&self) -> Option<String> {
        let Ok(mut guard) = self.output.lock() else {
            return None;
        };
        guard.take()
    }

    #[must_use]
    pub(crate) fn take_body(&self) -> Option<String> {
        let Ok(mut guard) = self.body.lock() else {
            return None;
        };
        guard.take()
    }
}

impl Clone for ActionView {
    fn clone(&self) -> Self {
        let output = self.output.lock().map_or(None, |guard| guard.clone());
        let body = self.body.lock().map_or(None, |guard| guard.clone());
        ActionView {
            lang: self.lang,
            edit_lines: self.edit_lines,
            read_offset: self.read_offset,
            read_limit: self.read_limit,
            read_path: self.read_path.clone(),
            computer: self.computer.clone(),
            edit_path: self.edit_path.clone(),
            output: std::sync::Mutex::new(output),
            body: std::sync::Mutex::new(body),
        }
    }
}
