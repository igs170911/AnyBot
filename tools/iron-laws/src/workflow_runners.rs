pub(crate) fn runner_violations(
    runs_on: Option<&serde_json::Value>,
    rel: &str,
    job: &str,
    out: &mut Vec<String>,
) {
    for label in runner_labels(runs_on) {
        if label_uses_latest(&label) {
            out.push(format!(
                "{rel}: job \"{job}\" runs-on \"{label}\" uses latest; pin the runner image (spec/09 K110)"
            ));
        }
    }
}

fn label_uses_latest(label: &str) -> bool {
    label
        .split('-')
        .any(|part| part.eq_ignore_ascii_case("latest"))
}

fn runner_labels(runs_on: Option<&serde_json::Value>) -> Vec<String> {
    match runs_on {
        Some(serde_json::Value::String(text)) => literal_or_matrix_labels(text),
        Some(serde_json::Value::Array(items)) => items
            .iter()
            .filter_map(|item| item.as_str())
            .flat_map(literal_or_matrix_labels)
            .collect(),
        _ => Vec::new(),
    }
}

fn literal_or_matrix_labels(text: &str) -> Vec<String> {
    if text.contains("${{") {
        Vec::new()
    } else {
        vec![text.to_owned()]
    }
}

pub(crate) fn matrix_runner_violations(
    job: &serde_json::Value,
    rel: &str,
    job_name: &str,
    out: &mut Vec<String>,
) {
    let Some(runs_on) = job.get("runs-on").and_then(|value| value.as_str()) else {
        return;
    };
    if !runs_on.contains("matrix.") {
        return;
    }
    let Some(include) = job
        .get("strategy")
        .and_then(|value| value.get("matrix"))
        .and_then(|value| value.get("include"))
        .and_then(|value| value.as_array())
    else {
        out.push(format!(
            "{rel}: job \"{job_name}\" runs-on uses a matrix expression but the matrix has no include list (spec/09 K110)"
        ));
        return;
    };
    let mut saw_runner = false;
    for row in include {
        let Some(label) = row.get("runner").and_then(|value| value.as_str()) else {
            continue;
        };
        saw_runner = true;
        if label_uses_latest(label) {
            out.push(format!(
                "{rel}: job \"{job_name}\" matrix runner \"{label}\" uses latest; pin the runner image (spec/09 K110)"
            ));
        }
    }
    if !saw_runner {
        out.push(format!(
            "{rel}: job \"{job_name}\" runs-on uses a matrix expression but no include row sets runner (spec/09 K110)"
        ));
    }
}
