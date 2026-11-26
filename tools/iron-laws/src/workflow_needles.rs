const NEEDLES: [&str; 23] = [
    "github.event.pull_request.title",
    "github.event.pull_request.body",
    "github.event.comment.body",
    "github.event.issue.title",
    "github.event.issue.body",
    "github.event.review.body",
    "github.event.head_commit.message",
    "github.event.head_commit.author.name",
    "github.event.head_commit.author.email",
    "github.event.pull_request.head.label",
    "github.head_ref",
    "github.event.pull_request.head.ref",
    "github.ref_name",
    "github.event.ref",
    "github.event.release.tag_name",
    "github.event.workflow_dispatch.inputs",
    "github.event.inputs",
    "inputs.",
    "github.event.workflow_call.inputs",
    "tojson(github.event)",
    "github.event.commits",
    "github.event.pull_request.head.sha",
    "github.event.comment.html_url",
];

#[must_use]
pub fn embeds_event_text(script: &str) -> bool {
    for chunk in script.split("${{") {
        let Some(expr) = chunk.split("}}").next() else {
            continue;
        };
        let compact = expr
            .split_whitespace()
            .collect::<String>()
            .to_ascii_lowercase();
        let dotted = bracket_to_dot(&compact);
        if needle_hits(&dotted) {
            return true;
        }
    }
    false
}

fn bracket_to_dot(expr: &str) -> String {
    expr.replace("['", ".")
        .replace("[\"", ".")
        .replace("']", "")
        .replace("\"]", "")
}

fn needle_hits(dotted: &str) -> bool {
    if NEEDLES.iter().any(|needle| dotted.contains(needle)) {
        return true;
    }
    dotted.contains("github.ref")
        && !dotted.contains("github.ref_name")
        && !dotted.contains("github.ref_protected")
}
