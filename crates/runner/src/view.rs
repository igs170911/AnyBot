use anybot_messages::{Catalog, Lang, MessageCode, Params};
use anybot_protocol::{ActualResult, GateRecord};
use anybot_trust::{ModelResultView, OpRejection, OutcomeText};

use crate::error::TurnError;

#[derive(Debug)]
pub(crate) struct ActionView {
    lang: Lang,
    edit_lines: u32,
    read_from: u32,
    read_to: u32,
    computer: String,
    edit_path: String,
    output: std::sync::Mutex<Option<String>>,
}

impl ActionView {
    #[must_use]
    pub(crate) fn new(lang: Lang) -> Self {
        ActionView {
            lang,
            edit_lines: 0,
            read_from: 1,
            read_to: 0,
            computer: String::new(),
            edit_path: String::new(),
            output: std::sync::Mutex::new(None),
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

    pub(crate) fn with_read_range(mut self, from: u32, to: u32) -> Self {
        self.read_from = from;
        self.read_to = to;
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
            ActualResult::Read(outcome) => OutcomeText::of(
                "tool-read-lines",
                vec![
                    ("path".to_owned(), outcome.path.clone()),
                    ("from".to_owned(), self.read_from.to_string()),
                    ("to".to_owned(), self.read_to.to_string()),
                ],
            ),
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

    #[must_use]
    pub(crate) fn take_output(&self) -> Option<String> {
        let Ok(mut guard) = self.output.lock() else {
            return None;
        };
        guard.take()
    }
}

fn command_text(exit_code: Option<i32>) -> OutcomeText {
    match exit_code {
        Some(0) | None => OutcomeText::of("tool-shell-no-output", Vec::new()),
        Some(code) => OutcomeText::of(
            "tool-shell-failed",
            vec![("code".to_owned(), code.to_string())],
        ),
    }
}

fn write_text(outcome: &anybot_protocol::WriteOutcome, lines: u32, requested: &str) -> OutcomeText {
    let shown = short_path(&outcome.path, requested);
    match outcome.before {
        anybot_protocol::BeforeDigest::Absent => OutcomeText::of(
            "tool-edit-created",
            vec![
                ("path".to_owned(), shown.clone()),
                ("lines".to_owned(), lines.to_string()),
            ],
        ),
        anybot_protocol::BeforeDigest::Present(_) => {
            OutcomeText::of("tool-edit-modified", vec![("path".to_owned(), shown)])
        }
    }
}

fn short_path(canonical: &str, requested: &str) -> String {
    if requested.is_empty() {
        return canonical.to_owned();
    }
    canonical
        .rsplit_once('/')
        .map_or(canonical.to_owned(), |(_, tail)| {
            if tail == requested.rsplit_once('/').map_or(requested, |(_, t)| t) {
                requested.to_owned()
            } else {
                canonical.to_owned()
            }
        })
}

pub(crate) fn render_text(lang: Lang, text: &OutcomeText) -> Result<String, TurnError> {
    let Some(code) = MessageCode::from_code(&text.code) else {
        return Err(TurnError::UnknownMessageCode {
            code: text.code.clone(),
        });
    };
    let mut params = Params::new();
    for (key, value) in &text.params {
        params = params.with_text(key, value);
    }
    let mut catalog = Catalog::new();
    catalog
        .format(lang, &code, &params)
        .map_err(TurnError::Messages)
}

impl Clone for ActionView {
    fn clone(&self) -> Self {
        let output = self.output.lock().map_or(None, |guard| guard.clone());
        ActionView {
            lang: self.lang,
            edit_lines: self.edit_lines,
            read_from: self.read_from,
            read_to: self.read_to,
            computer: self.computer.clone(),
            edit_path: self.edit_path.clone(),
            output: std::sync::Mutex::new(output),
        }
    }
}
