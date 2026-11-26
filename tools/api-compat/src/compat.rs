use std::path::Path;

use crate::log;
use crate::sha;

type ReadRefFn = dyn Fn(&Path, &str, &str) -> Result<Option<String>, String>;
type TrackedFn = dyn Fn(&Path, &str) -> Result<(), String>;
type TagsFn = dyn Fn(&Path) -> Result<Vec<String>, String>;
type BreakingFn = dyn Fn(&str, &str) -> Vec<String>;

pub struct Inputs<'a> {
    pub repo: &'a Path,
    pub git_ref: &'a str,
    pub read_ref: &'a ReadRefFn,
    pub tracked: &'a TrackedFn,
    pub release_tags: &'a TagsFn,
    pub breaking_of: &'a BreakingFn,
}

#[derive(Clone)]
pub struct Baselines {
    pub openapi: Vec<u8>,
    pub asyncapi_text: String,
    pub log_text: String,
}

#[must_use = "the violations decide the gate verdict"]
pub fn violations(inputs: &Inputs<'_>) -> Vec<String> {
    violations_with_notes(inputs).0
}

#[must_use = "the violations and the breaking-change notes decide the gate verdict"]
pub fn violations_with_notes(inputs: &Inputs<'_>) -> (Vec<String>, Vec<String>) {
    let mut out = Vec::new();
    let mut notes = Vec::new();
    if let Err(e) = (inputs.tracked)(inputs.repo, sha::CHANGELOG_REL) {
        out.push(e);
        return (out, notes);
    }
    match (inputs.release_tags)(inputs.repo) {
        Ok(tags) => {
            if let Some(tag) = tags.first() {
                out.push(format!(
                    "a released version {tag} exists; the post-release compatibility rules are not implemented yet, so any baseline change blocks (spec/09 K118 6)"
                ));
                return (out, notes);
            }
        }
        Err(e) => {
            out.push(e);
            return (out, notes);
        }
    }
    let tree = match read_tree(inputs.repo) {
        Ok(files) => files,
        Err(e) => return (vec![e], notes),
    };
    let ref_files = match read_ref_baselines(inputs, inputs.git_ref) {
        Ok(files) => files,
        Err(e) => return (e, notes),
    };
    out.extend(gate_violations(
        inputs,
        &ref_files,
        &tree,
        inputs.git_ref,
        &mut notes,
    ));
    (out, notes)
}

#[must_use = "the tree files feed the gate, errors carry the reason"]
fn read_tree(repo: &Path) -> Result<Baselines, String> {
    let openapi = sha::read_bytes(&repo.join(sha::OPENAPI_REL))?;
    let asyncapi = sha::read_bytes(&repo.join(sha::ASYNCAPI_REL))?;
    let log_text = std::fs::read_to_string(repo.join(sha::CHANGELOG_REL))
        .map_err(|e| format!("cannot read {}: {e}", sha::CHANGELOG_REL))?;
    Ok(Baselines {
        openapi,
        asyncapi_text: String::from_utf8_lossy(&asyncapi).into_owned(),
        log_text,
    })
}

fn read_ref_baselines(inputs: &Inputs<'_>, git_ref: &str) -> Result<Baselines, Vec<String>> {
    let mut missing = Vec::new();
    let openapi = read_ref_slot(inputs, git_ref, sha::OPENAPI_REL, &mut missing);
    let asyncapi_text = read_ref_slot(inputs, git_ref, sha::ASYNCAPI_REL, &mut missing);
    let log_slot = read_ref_slot_log(inputs, git_ref, &mut missing);
    if !missing.is_empty() {
        return Err(missing);
    }
    Ok(Baselines {
        openapi: match openapi {
            Some(text) => text.into_bytes(),
            None => Vec::new(),
        },
        asyncapi_text: asyncapi_text.unwrap_or_default(),
        log_text: log_slot.unwrap_or_default(),
    })
}

fn read_ref_slot(
    inputs: &Inputs<'_>,
    git_ref: &str,
    rel: &str,
    missing: &mut Vec<String>,
) -> Option<String> {
    match (inputs.read_ref)(inputs.repo, git_ref, rel) {
        Ok(Some(text)) => Some(text),
        Ok(None) => {
            missing.push(missing_baseline(git_ref, rel));
            None
        }
        Err(e) => {
            missing.push(e);
            None
        }
    }
}

fn read_ref_slot_log(
    inputs: &Inputs<'_>,
    git_ref: &str,
    missing: &mut Vec<String>,
) -> Option<String> {
    match (inputs.read_ref)(inputs.repo, git_ref, sha::CHANGELOG_REL) {
        Ok(Some(text)) => Some(text),
        Ok(None) => Some(String::new()),
        Err(e) => {
            missing.push(e);
            None
        }
    }
}

fn gate_violations(
    inputs: &Inputs<'_>,
    ref_files: &Baselines,
    tree: &Baselines,
    git_ref: &str,
    notes: &mut Vec<String>,
) -> Vec<String> {
    let mut out = baseline_file_violations(tree);
    out.extend(log_prefix_violations(&ref_files.log_text, &tree.log_text));
    let grew = log::entries(&tree.log_text).len() > log::entries(&ref_files.log_text).len();
    let baselines_changed =
        tree.openapi != ref_files.openapi || tree.asyncapi_text != ref_files.asyncapi_text;
    if baselines_changed && !grew {
        out.push(format!(
            "{}: the committed baselines changed; api/breaking-changes.log must grow with a dated record of what changed and why (spec/09 K118 6)",
            sha::CHANGELOG_REL
        ));
    }
    for item in (inputs.breaking_of)(&ref_files.asyncapi_text, &tree.asyncapi_text) {
        notes.push(format!(
            "{}: breaking protocol change: {item} (record it before merging {git_ref})",
            sha::ASYNCAPI_REL
        ));
    }
    out
}

fn baseline_file_violations(tree: &Baselines) -> Vec<String> {
    let mut out = log::format_violations(sha::CHANGELOG_REL, &tree.log_text);
    let baseline = sha::baseline(&tree.openapi, tree.asyncapi_text.as_bytes());
    let entries = log::entries(&tree.log_text);
    if entries.is_empty() {
        out.push(format!(
            "{}: the record is empty; every baseline change needs a dated line (spec/09 K118 6)",
            sha::CHANGELOG_REL
        ));
        return out;
    }
    let Some(last) = entries.last() else {
        return out;
    };
    if !sha::is_hex64(&last.hash) {
        out.push(format!(
            "{}: a changelog line carries a hash that is not 64 hex digits",
            sha::CHANGELOG_REL
        ));
    } else if last.hash != baseline {
        out.push(format!(
            "{}: the last changelog line names sha256={} but the committed baselines hash to {}; a regenerated baseline must record its own hash",
            sha::CHANGELOG_REL, last.hash, baseline
        ));
    }
    out
}

fn log_prefix_violations(ref_log: &str, tree_log: &str) -> Vec<String> {
    let mut out = Vec::new();
    if ref_log.is_empty() || tree_log.starts_with(ref_log) {
        return out;
    }
    out.push(format!(
        "{}: the record only appends; the ref version of the log must be a byte-for-byte prefix of the current log, old lines never change (spec/09 K118 6)",
        sha::CHANGELOG_REL
    ));
    out
}

fn missing_baseline(git_ref: &str, rel: &str) -> String {
    format!(
        "{rel} is missing at ref {git_ref}; commit the baselines first (the api-docs gate keeps them current)"
    )
}
