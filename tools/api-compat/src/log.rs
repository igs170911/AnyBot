pub struct Entry {
    pub date: String,
    pub hash: String,
    pub what: String,
    pub why: String,
}

#[must_use = "the parsed changelog lines feed the growth and hash checks"]
pub fn entries(text: &str) -> Vec<Entry> {
    let mut out = Vec::new();
    for line in text.lines() {
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        out.push(parse(line));
    }
    out
}

#[must_use = "malformed changelog lines block the gate"]
pub fn format_violations(rel: &str, text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in text.lines() {
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let entry = parse(line);
        if !valid_date(&entry.date) {
            out.push(format!(
                "{rel}: a changelog line has no YYYY-MM-DD date: {line}"
            ));
        }
        if entry.what.is_empty() || entry.why.is_empty() {
            out.push(format!(
                "{rel}: a changelog line needs a what part and a why part: {line}"
            ));
        }
    }
    out
}

fn parse(line: &str) -> Entry {
    let parts: Vec<&str> = line.split(" | ").map(str::trim).collect();
    Entry {
        date: parts.first().copied().unwrap_or_default().to_owned(),
        hash: parts
            .get(1)
            .and_then(|p| p.strip_prefix("sha256="))
            .unwrap_or_default()
            .to_owned(),
        what: parts.get(2).copied().unwrap_or_default().to_owned(),
        why: parts.get(3).copied().unwrap_or_default().to_owned(),
    }
}

fn digit(c: char) -> bool {
    c.is_ascii_digit()
}

#[must_use = "the date part decides whether a changelog line is valid"]
fn valid_date(text: &str) -> bool {
    let chars: Vec<char> = text.chars().collect();
    let Some(first4) = chars.get(0..4) else {
        return false;
    };
    chars.len() == 10
        && first4.iter().copied().all(digit)
        && chars.get(4) == Some(&'-')
        && chars.get(5).copied().is_some_and(digit)
        && chars.get(6).copied().is_some_and(digit)
        && chars.get(7) == Some(&'-')
        && chars.get(8).copied().is_some_and(digit)
        && chars.get(9).copied().is_some_and(digit)
}
