#[must_use = "the stripped text is split into statements"]
pub fn strip_sql_comments(content: &str) -> String {
    let mut out = String::with_capacity(content.len());
    let bytes = content.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes.get(i) == Some(&b'-') && bytes.get(i.saturating_add(1)) == Some(&b'-') {
            while bytes.get(i).is_some_and(|b| *b != b'\n') {
                i = i.saturating_add(1);
            }
            continue;
        }
        if bytes.get(i) == Some(&b'/') && bytes.get(i.saturating_add(1)) == Some(&b'*') {
            i = i.saturating_add(2);
            while i < bytes.len() {
                if bytes.get(i) == Some(&b'*') && bytes.get(i.saturating_add(1)) == Some(&b'/') {
                    i = i.saturating_add(2);
                    break;
                }
                i = i.saturating_add(1);
            }
            continue;
        }
        if bytes.get(i) == Some(&b'\'') {
            out.push('\'');
            i = i.saturating_add(1);
            while let Some(b) = bytes.get(i) {
                if *b == b'\n' {
                    out.push('\n');
                    i = i.saturating_add(1);
                    continue;
                }
                if *b == b'\'' {
                    out.push('\'');
                    i = i.saturating_add(1);
                    if bytes.get(i) == Some(&b'\'') {
                        out.push('\'');
                        i = i.saturating_add(1);
                        continue;
                    }
                    break;
                }
                out.push(' ');
                i = i.saturating_add(1);
            }
            continue;
        }
        let Some(byte) = bytes.get(i) else {
            break;
        };
        out.push(char::from(*byte));
        i = i.saturating_add(1);
    }
    out
}

#[must_use = "statements carry the line the checker reports"]
pub struct Statement {
    pub start_line: usize,
    pub text: String,
}

#[must_use]
pub fn statements(stripped: &str) -> Vec<Statement> {
    let mut out = Vec::new();
    let mut buffer = String::new();
    let mut start_line = 0usize;
    for (index, line) in stripped.lines().enumerate() {
        let number = index.saturating_add(1);
        if buffer.is_empty() && !line.trim().is_empty() {
            start_line = number;
        }
        buffer.push_str(line);
        buffer.push('\n');
        if line.contains(';') {
            push_statement(&mut out, &buffer, start_line);
            buffer.clear();
        }
    }
    push_statement(&mut out, &buffer, start_line);
    out
}

fn push_statement(out: &mut Vec<Statement>, buffer: &str, start_line: usize) {
    if buffer.trim().is_empty() {
        return;
    }
    out.push(Statement {
        start_line,
        text: buffer.replace(';', " "),
    });
}
