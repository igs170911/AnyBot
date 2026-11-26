pub const CODE_TOOL_UNREADABLE_SHAPE: &str = "tool-unreadable-shape";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LexWord {
    Word,
    Assign,
    Operator,
    Opaque,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexToken {
    pub kind: LexWord,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LexLine {
    Commands(Vec<Vec<LexToken>>),
    Opaque,
}

#[must_use]
pub fn lex_line(line: &str) -> LexLine {
    let mut words = Vec::new();
    let mut commands = Vec::new();
    let mut rest = line;
    while !rest.is_empty() {
        rest = skip_space(rest);
        if rest.is_empty() {
            break;
        }
        if let Some(operator) = operator_at(rest) {
            if !push_command(&mut words, &mut commands) {
                return LexLine::Opaque;
            }
            rest = rest.get(operator.len()..).unwrap_or("");
            continue;
        }
        let Some((token, next)) = take_word(rest) else {
            return LexLine::Opaque;
        };
        if token.kind == LexWord::Opaque {
            return LexLine::Opaque;
        }
        words.push(token);
        rest = next;
    }
    if !push_command(&mut words, &mut commands) {
        return LexLine::Opaque;
    }
    LexLine::Commands(commands)
}

fn push_command(words: &mut Vec<LexToken>, commands: &mut Vec<Vec<LexToken>>) -> bool {
    if words.is_empty() {
        return true;
    }
    let command = std::mem::take(words);
    if command.iter().any(|token| token.kind == LexWord::Opaque)
        || command.iter().all(|token| token.kind == LexWord::Assign)
    {
        return false;
    }
    commands.push(command);
    true
}

fn skip_space(text: &str) -> &str {
    text.trim_start_matches([' ', '\t', '\r'])
}

fn operator_at(text: &str) -> Option<&str> {
    if starts_opaque_run(text) {
        return None;
    }
    if text.starts_with('\n') || text.starts_with(';') {
        return Some(if text.starts_with('\n') { "\n" } else { ";" });
    }
    ["&&", "||", "|", "&"]
        .into_iter()
        .find(|token| text.starts_with(token) && operator_is_separated(text, token.len()))
}

fn operator_is_separated(text: &str, len: usize) -> bool {
    let rest = text.get(len..).unwrap_or("");
    rest.is_empty() || rest.starts_with([' ', '\t', '\n', '\r', ';'])
}

fn take_word(text: &str) -> Option<(LexToken, &str)> {
    if starts_opaque_run(text) {
        let end = opaque_run_end(text);
        let raw = text.get(..end)?;
        return Some((
            LexToken {
                kind: LexWord::Opaque,
                text: raw.to_owned(),
            },
            text.get(end..).unwrap_or(""),
        ));
    }
    let bytes = text.as_bytes();
    let first = *bytes.first()?;
    if first == b'\'' {
        return quoted_word(text, '\'');
    }
    if first == b'"' {
        return quoted_word(text, '"');
    }
    let mut end = 0usize;
    let mut saw_quote = false;
    while let Some(&byte) = bytes.get(end) {
        if is_word_end(byte) {
            break;
        }
        if byte == b'\'' || byte == b'"' {
            let quote = if byte == b'\'' { '\'' } else { '"' };
            let rest = text.get(end..)?;
            let (_, next) = quoted_span(rest, quote)?;
            saw_quote = true;
            end = text.len().saturating_sub(next.len());
            continue;
        }
        end = end.saturating_add(1);
    }
    if end == 0 {
        return None;
    }
    let raw = text.get(..end)?;
    let next = text.get(end..).unwrap_or("");
    Some((word_token(raw, saw_quote), next))
}

fn quoted_word(text: &str, quote: char) -> Option<(LexToken, &str)> {
    let (inner, next) = quoted_span(text, quote)?;
    let kind = if quoted_has_opaque(inner) {
        LexWord::Opaque
    } else {
        LexWord::Word
    };
    Some((
        LexToken {
            kind,
            text: inner.to_owned(),
        },
        next,
    ))
}

fn quoted_span(text: &str, quote: char) -> Option<(&str, &str)> {
    let mut chars = text.chars();
    if chars.next() != Some(quote) {
        return None;
    }
    let mut end = quote.len_utf8();
    for ch in chars {
        end = end.saturating_add(ch.len_utf8());
        if ch == quote {
            let inner = text.get(quote.len_utf8()..end.saturating_sub(quote.len_utf8()))?;
            let next = text.get(end..).unwrap_or("");
            return Some((inner, next));
        }
    }
    None
}

fn word_token(raw: &str, saw_quote: bool) -> LexToken {
    let text = if saw_quote {
        strip_quotes(raw)
    } else {
        raw.to_owned()
    };
    let kind = if !saw_quote && is_assignment(&text) {
        LexWord::Assign
    } else {
        LexWord::Word
    };
    LexToken { kind, text }
}

fn strip_quotes(raw: &str) -> String {
    let mut out = String::new();
    let mut quote = None;
    for ch in raw.chars() {
        if quote == Some(ch) {
            quote = None;
            continue;
        }
        if quote.is_none() && (ch == '\'' || ch == '"') {
            quote = Some(ch);
            continue;
        }
        out.push(ch);
    }
    out
}

fn is_assignment(text: &str) -> bool {
    let Some((name, _)) = text.split_once('=') else {
        return false;
    };
    let mut chars = name.chars();
    match chars.next() {
        Some(first) if first.is_ascii_alphabetic() || first == '_' => {}
        _ => return false,
    }
    chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

fn starts_opaque_run(text: &str) -> bool {
    let bytes = text.as_bytes();
    matches!(
        bytes.first().copied(),
        Some(b'(' | b')' | b'{' | b'}' | b'!' | b'\\' | b'`' | b'$' | b'<' | b'>')
    ) || text.starts_with("2>")
        || bytes.get(..2) == Some(b"$(")
        || bytes.get(..2) == Some(b"${")
        || bytes.get(..2) == Some(b"<(")
        || bytes.get(..2) == Some(b">(")
}

fn opaque_run_end(text: &str) -> usize {
    let bytes = text.as_bytes();
    let mut end = 1usize;
    while bytes.get(end).is_some_and(|byte| !is_word_end(*byte)) {
        end = end.saturating_add(1);
    }
    end
}

fn is_word_end(byte: u8) -> bool {
    matches!(
        byte,
        b' ' | b'\t'
            | b'\r'
            | b'\n'
            | b';'
            | b'|'
            | b'&'
            | b'('
            | b')'
            | b'{'
            | b'}'
            | b'!'
            | b'\\'
            | b'`'
            | b'$'
            | b'<'
            | b'>'
    )
}

fn quoted_has_opaque(text: &str) -> bool {
    text.bytes().any(|byte| {
        matches!(
            byte,
            b'(' | b')' | b'{' | b'}' | b'!' | b'\\' | b'`' | b'$' | b'<' | b'>'
        )
    })
}
