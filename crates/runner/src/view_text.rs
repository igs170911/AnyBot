use anybot_messages::{Catalog, Lang, MessageCode, Params};
use anybot_protocol::{BeforeDigest, ListOutcome, ReadOutcome, TargetKind, WriteOutcome};
use anybot_tools::list::{list_empty_code, list_more_code, render_listing};
use anybot_tools::read::{block_of, format_numbered, numbered_lines, read_code, read_more_code};
use anybot_trust::OutcomeText;

use crate::error::TurnError;

pub(crate) fn command_text(exit_code: Option<i32>) -> OutcomeText {
    match exit_code {
        Some(0) | None => OutcomeText::of("tool-shell-no-output", Vec::new()),
        Some(code) => OutcomeText::of(
            "tool-shell-failed",
            vec![("code".to_owned(), code.to_string())],
        ),
    }
}

pub(crate) fn write_text(outcome: &WriteOutcome, lines: u32, requested: &str) -> OutcomeText {
    let shown = short_path(&outcome.path, requested);
    match outcome.before {
        BeforeDigest::Absent => OutcomeText::of(
            "tool-edit-created",
            vec![
                ("path".to_owned(), shown.clone()),
                ("lines".to_owned(), lines.to_string()),
            ],
        ),
        BeforeDigest::Present(_) => {
            OutcomeText::of("tool-edit-modified", vec![("path".to_owned(), shown)])
        }
    }
}

pub(crate) fn read_text(
    lang: Lang,
    outcome: &ReadOutcome,
    offset: usize,
    limit: usize,
    requested: &str,
) -> (OutcomeText, Option<String>) {
    let shown = short_path(&outcome.path, requested);
    if let Some(block) = block_of(
        outcome.size,
        outcome.kind == TargetKind::Directory,
        &outcome.content,
    ) {
        return (
            OutcomeText::of(block.code(), vec![("path".to_owned(), shown)]),
            None,
        );
    }
    let numbered = numbered_lines(&outcome.content, offset, limit);
    let to = if numbered.lines.is_empty() {
        numbered.from.saturating_sub(1)
    } else {
        numbered
            .from
            .saturating_add(numbered.lines.len())
            .saturating_sub(1)
    };
    let mut body = format_numbered(&numbered);
    if numbered.remaining > 0 {
        let more = OutcomeText::of(
            read_more_code(),
            vec![
                ("remaining".to_owned(), numbered.remaining.to_string()),
                ("next_offset".to_owned(), numbered.next_offset.to_string()),
            ],
        );
        if let Ok(line) = render_text(lang, &more) {
            if !body.is_empty() {
                body.push('\n');
            }
            body.push_str(&line);
        }
    }
    let header = OutcomeText::of(
        read_code(),
        vec![
            ("path".to_owned(), shown),
            ("from".to_owned(), numbered.from.to_string()),
            ("to".to_owned(), to.to_string()),
        ],
    );
    (header, Some(body))
}

pub(crate) fn list_text(lang: Lang, outcome: &ListOutcome) -> (OutcomeText, Option<String>) {
    let listing = render_listing(&outcome.entries);
    if listing.is_empty {
        return (OutcomeText::of(list_empty_code(), Vec::new()), None);
    }
    let mut body = listing.lines.join("\n");
    if listing.remaining > 0 {
        let more = OutcomeText::of(
            list_more_code(),
            vec![("count".to_owned(), listing.remaining.to_string())],
        );
        if let Ok(line) = render_text(lang, &more) {
            body.push('\n');
            body.push_str(&line);
        }
    }
    (
        OutcomeText::of(anybot_trust::CODE_TOOL_EMPTY, Vec::new()),
        Some(body),
    )
}

pub(crate) fn short_path(canonical: &str, requested: &str) -> String {
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
