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
        serde_json::Value::Null
        | serde_json::Value::Bool(_)
        | serde_json::Value::Number(_)
        | serde_json::Value::String(_) => {}
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
    match doc.get("permissions") {
        None => out.push(format!(
            "{rel}: the workflow has no top-level permissions block; grant least privilege (spec/09 K110)"
        )),
        Some(permissions) => top_level_permission_violations(permissions, rel, &mut out),
    }
    let triggers_pr = trigger_includes_pull_request(doc.get("on"));
    let jobs = doc.get("jobs");
    for (name, job) in jobs.and_then(|j| j.as_object()).into_iter().flatten() {
        match caller_job_shape(job) {
            CallerShape::Caller => {
                if job.get("timeout-minutes").is_some() {
                    out.push(format!(
                        "{rel}: job \"{name}\" calls a workflow and sets timeout-minutes; a caller job cannot set it (spec/09 K110)"
                    ));
                }
            }
            CallerShape::Local => {
                if job.get("timeout-minutes").is_none() {
                    out.push(format!(
                        "{rel}: job \"{name}\" has no timeout-minutes (spec/09 K110)"
                    ));
                }
            }
        }
        if let Some(permissions) = job.get("permissions")
            && triggers_pr
            && permissions_grant_write(permissions)
            && job_runs_on_pull_request(job)
        {
            out.push(format!(
                "{rel}: job \"{name}\" grants a write permission and the workflow triggers on pull_request or pull_request_target (spec/09 K111)"
            ));
        }
        crate::workflow_runners::runner_violations(job.get("runs-on"), rel, name, &mut out);
        crate::workflow_runners::matrix_runner_violations(job, rel, name, &mut out);
        script_injection_violations(job, rel, name, &mut out);
    }
    out
}

fn top_level_permission_violations(
    permissions: &serde_json::Value,
    rel: &str,
    out: &mut Vec<String>,
) {
    let read_only = matches!(permissions, serde_json::Value::Object(map)
        if map.len() == 1 && map.get("contents").and_then(|v| v.as_str()) == Some("read"));
    if !read_only {
        out.push(format!(
            "{rel}: top-level permissions must be exactly contents: read; put write on the one job that needs it (spec/09 K110)"
        ));
    }
}

fn permission_is_write(text: &str) -> bool {
    text == "write" || text == "write-all"
}

fn permissions_grant_write(permissions: &serde_json::Value) -> bool {
    match permissions {
        serde_json::Value::String(text) => permission_is_write(text),
        serde_json::Value::Object(map) => map
            .values()
            .any(|value| value.as_str().is_some_and(permission_is_write)),
        serde_json::Value::Null
        | serde_json::Value::Bool(_)
        | serde_json::Value::Number(_)
        | serde_json::Value::Array(_) => false,
    }
}

fn trigger_includes_pull_request(on: Option<&serde_json::Value>) -> bool {
    match on {
        Some(serde_json::Value::String(text)) => is_pull_request_event(text),
        Some(serde_json::Value::Array(items)) => items
            .iter()
            .any(|item| item.as_str().is_some_and(is_pull_request_event)),
        Some(serde_json::Value::Object(map)) => map.keys().any(|key| is_pull_request_event(key)),
        _ => false,
    }
}

fn is_pull_request_event(text: &str) -> bool {
    text == "pull_request" || text == "pull_request_target"
}

fn job_runs_on_pull_request(job: &serde_json::Value) -> bool {
    let Some(cond) = job.get("if").and_then(|value| value.as_str()) else {
        return true;
    };
    !if_requires_non_pull_request(cond)
}

fn if_requires_non_pull_request(cond: &str) -> bool {
    let compact = cond.split_whitespace().collect::<String>();
    let requires_push = compact.contains("github.event_name=='push'")
        || compact.contains("github.event_name==\"push\"");
    requires_push && !compact.contains("pull_request") && !compact.contains("||")
}

enum CallerShape {
    Caller,
    Local,
}

fn caller_job_shape(job: &serde_json::Value) -> CallerShape {
    if job.get("uses").and_then(|value| value.as_str()).is_some() {
        CallerShape::Caller
    } else {
        CallerShape::Local
    }
}

fn script_injection_violations(
    job: &serde_json::Value,
    rel: &str,
    job_name: &str,
    out: &mut Vec<String>,
) {
    let Some(steps) = job.get("steps").and_then(|steps| steps.as_array()) else {
        return;
    };
    for step in steps {
        let Some(script) = step.get("run").and_then(|run| run.as_str()) else {
            continue;
        };
        if script_embeds_event_text(script) {
            out.push(format!(
                "{rel}: job \"{job_name}\" puts a title, branch name, or comment straight into run:; assign it to env: first (spec/09 K110)"
            ));
        }
    }
}

fn script_embeds_event_text(script: &str) -> bool {
    crate::workflow_needles::embeds_event_text(script)
}

#[must_use]
pub fn violations(repo: &Path) -> Vec<String> {
    let mut out = Vec::new();
    if !repo.join(WORKFLOWS_REL).is_dir() {
        out.push(format!(
            "{WORKFLOWS_REL}: missing; workflow pins cannot be checked (spec/09 K110)"
        ));
        return out;
    }
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
