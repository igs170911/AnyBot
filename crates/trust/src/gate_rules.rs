use crate::action::{ActionKey, ActionSubject, GateAction};
use crate::always_list::AlwaysList;
use crate::gate_engine::Opinion;
use crate::gate_flow::{RuleEntry, UserRules};
use crate::gate_full::StyleFlags;
use crate::machine_rules::is_outside_write;
use crate::pattern::{PatternTarget, RulePattern, allow_hits, ask_hits, pattern_text};
use crate::sandbox_levels::all_segments_always_covered;

pub(crate) fn style_hits(
    action: &GateAction,
    computer: &str,
    user: &UserRules,
    home: &str,
) -> StyleFlags {
    let key = action.key();
    let target = pattern_target(action);
    StyleFlags {
        ask_style_hit: user.ask_style.iter().any(|k| k == &key)
            || any_pattern(&user.ask_patterns, target.as_ref(), home, ask_hits),
        allow_style_hit: user.allow_style.iter().any(|k| k == &key)
            || any_pattern(&user.allow_patterns, target.as_ref(), home, allow_hits),
        always_hit: always_hit(action, computer, &user.always),
        session_allowed: user.session.iter().any(|k| k == &key),
    }
}

pub(crate) fn hook_opinion(entries: &[RuleEntry], key: &ActionKey) -> Opinion {
    entries
        .iter()
        .find(|e| &e.key == key)
        .map_or(Opinion::None, |e| e.opinion)
}

pub(crate) fn ask_style_label(action: &GateAction, user: &UserRules, home: &str) -> Option<String> {
    let key = action.key();
    if user.ask_style.iter().any(|k| k == &key) {
        let (tool, subject) = key.parts();
        return Some(format!("{tool}({subject})"));
    }
    let target = pattern_target(action)?;
    user.ask_patterns
        .iter()
        .find(|p| ask_hits(p, &target, home))
        .map(pattern_text)
}

fn any_pattern(
    patterns: &[RulePattern],
    target: Option<&PatternTarget>,
    home: &str,
    hits: fn(&RulePattern, &PatternTarget, &str) -> bool,
) -> bool {
    target.is_some_and(|t| patterns.iter().any(|p| hits(p, t, home)))
}

fn pattern_target(action: &GateAction) -> Option<PatternTarget> {
    match &action.subject {
        ActionSubject::Command { line, .. } => Some(PatternTarget::Command { line: line.clone() }),
        ActionSubject::ReadFile { path }
        | ActionSubject::WriteFile { path, .. }
        | ActionSubject::EditFile { path, .. }
        | ActionSubject::PathCheck { path } => {
            path.resolved_text().map(|resolved| PatternTarget::Path {
                path: resolved.to_owned(),
            })
        }
        ActionSubject::Capabilities => None,
    }
}

fn always_hit(action: &GateAction, computer: &str, always: &AlwaysList) -> bool {
    if let ActionSubject::Command { line, .. } = &action.subject {
        return all_segments_always_covered(line, always, computer);
    }
    always_keys(action, computer)
        .iter()
        .any(|k| always.contains(k))
}

fn always_keys(action: &GateAction, computer: &str) -> Vec<String> {
    match &action.subject {
        ActionSubject::Command { .. } | ActionSubject::Capabilities => Vec::new(),
        ActionSubject::WriteFile { path, .. } if !is_outside_write(path, &action.workdir) => {
            vec![format!("write workspace file on {computer}")]
        }
        ActionSubject::ReadFile { path }
        | ActionSubject::WriteFile { path, .. }
        | ActionSubject::EditFile { path, .. }
        | ActionSubject::PathCheck { path } => {
            let effective = path.effective(&action.workdir);
            vec![
                format!("read file {effective} on {computer}"),
                format!("edit file {effective} on {computer}"),
            ]
        }
    }
}
