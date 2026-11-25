use std::path::Path;

use crate::collect::dir_name;
use crate::collect::path_to_rel;
use crate::collect::read_file_lossy;

const WORKFLOWS_REL: &str = ".github/workflows";

#[must_use]
pub fn is_workflow_file(name: &str) -> bool {
    name.rsplit_once('.')
        .is_some_and(|(_, ext)| ext.eq_ignore_ascii_case("yml") || ext.eq_ignore_ascii_case("yaml"))
}

fn workflow_rels(repo: &Path) -> Vec<String> {
    let mut rels = Vec::new();
    let dir = repo.join(WORKFLOWS_REL);
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return rels;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            continue;
        }
        let Some(name) = dir_name(&path) else {
            continue;
        };
        if is_workflow_file(&name)
            && let Some(rel) = path_to_rel(&path, repo)
        {
            rels.push(rel);
        }
    }
    rels.sort();
    rels
}

fn uses_violations(node: &serde_json::Value, rel: &str, out: &mut Vec<String>) {
    match node {
        serde_json::Value::Object(map) => {
            for (key, value) in map {
                if key == "uses" {
                    if let serde_json::Value::String(text) = value
                        && !use_ref_is_pinned(text)
                    {
                        out.push(format!(
                            "{rel}: uses \"{text}\" is not pinned to a full 40-hex commit hash (spec/09 K110)"
                        ));
                    }
                    continue;
                }
                uses_violations(value, rel, out);
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                uses_violations(item, rel, out);
            }
        }
        serde_json::Value::Null | serde_json::Value::Bool(_) | serde_json::Value::Number(_) | serde_json::Value::String(_) => {}
    }
}

#[must_use]
pub fn use_ref_is_pinned(text: &str) -> bool {
    if text.starts_with("./") {
        return true;
    }
    let Some((path, hash)) = text.split_once('@') else {
        return false;
    };
    !path.is_empty()
        && hash.len() == 40
        && hash
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
}

fn job_violations(doc: &serde_json::Value, rel: &str) -> Vec<String> {
    let mut out = Vec::new();
    if doc.get("permissions").is_none() {
        out.push(format!(
            "{rel}: the workflow has no top-level permissions block; grant least privilege (spec/09 K110)"
        ));
    }
    let jobs = doc.get("jobs");
    for (name, job) in jobs.and_then(|j| j.as_object()).into_iter().flatten() {
        if job.get("timeout-minutes").is_none() {
            out.push(format!(
                "{rel}: job \"{name}\" has no timeout-minutes (spec/09 K110)"
            ));
        }
    }
    out
}

#[must_use]
pub fn violations(repo: &Path) -> Vec<String> {
    let mut out = Vec::new();
    for rel in workflow_rels(repo) {
        let Ok(text) = read_file_lossy(&repo.join(&rel)) else {
            out.push(format!("{rel}: cannot be read"));
            continue;
        };
        let Ok(doc) = yaml_serde::from_str::<serde_json::Value>(&text) else {
            out.push(format!("{rel}: is not valid YAML (spec/09 K110)"));
            continue;
        };
        out.extend(job_violations(&doc, &rel));
        uses_violations(&doc, &rel, &mut out);
    }
    out
}

#[cfg(test)]
#[path = "workflows_tests.rs"]
mod tests;
