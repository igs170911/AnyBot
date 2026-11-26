#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditKind {
    Created,
    Overwritten,
    Modified,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditApplied {
    pub content: String,
    pub kind: EditKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditRejection {
    NotFound,
    Multiple { hits: usize },
    FileMissing,
}

#[must_use]
pub fn line_count(content: &str) -> usize {
    content.lines().count()
}

pub fn apply_edit(
    current: Option<&str>,
    old_text: &str,
    new_text: &str,
) -> Result<EditApplied, EditRejection> {
    if old_text.is_empty() {
        return Ok(overwrite(current, new_text));
    }
    let Some(existing) = current else {
        return Err(EditRejection::FileMissing);
    };
    match occurrences(existing, old_text) {
        0 => Err(EditRejection::NotFound),
        1 => Ok(EditApplied {
            content: replace_once(existing, old_text, new_text),
            kind: EditKind::Modified,
        }),
        hits => Err(EditRejection::Multiple { hits }),
    }
}

fn overwrite(current: Option<&str>, new_text: &str) -> EditApplied {
    EditApplied {
        content: new_text.to_owned(),
        kind: match current {
            Some(_) => EditKind::Overwritten,
            None => EditKind::Created,
        },
    }
}

#[must_use]
pub fn occurrences(text: &str, needle: &str) -> usize {
    if needle.is_empty() {
        return 0;
    }
    text.match_indices(needle).count()
}

#[must_use]
pub fn replace_once(text: &str, needle: &str, replacement: &str) -> String {
    let Some(pos) = text.find(needle) else {
        return text.to_owned();
    };
    let before = text.split_at(pos).0;
    let after = text.split_at(pos.saturating_add(needle.len())).1;
    let mut out = String::new();
    out.push_str(before);
    out.push_str(replacement);
    out.push_str(after);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_edit_rule_requires_one_occurrence_of_the_old_text() {
        assert_eq!(
            apply_edit(Some("a b a"), "a", "z"),
            Err(EditRejection::Multiple { hits: 2 })
        );
        assert_eq!(
            apply_edit(Some("one two"), "three", "x"),
            Err(EditRejection::NotFound)
        );
        assert_eq!(
            apply_edit(None, "needle", "x"),
            Err(EditRejection::FileMissing)
        );
    }

    #[test]
    fn test_edit_rule_writes_the_replaced_content_and_names_the_kind() {
        let applied = apply_edit(Some("alpha beta"), "beta", "gamma").unwrap();
        assert_eq!(applied.content, "alpha gamma");
        assert_eq!(applied.kind, EditKind::Modified);
        let created = apply_edit(None, "", "brand new").unwrap();
        assert_eq!(created.kind, EditKind::Created);
        let overwritten = apply_edit(Some("was here"), "", "replaced").unwrap();
        assert_eq!(overwritten.kind, EditKind::Overwritten);
    }

    #[test]
    fn test_edit_rule_counts_written_lines() {
        assert_eq!(line_count("one\ntwo\nthree"), 3);
        assert_eq!(line_count(""), 0);
    }
}
