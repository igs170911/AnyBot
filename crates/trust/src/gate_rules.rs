use crate::action::{ActionKey, ActionSubject, GateAction};
use crate::always_list::AlwaysList;
use crate::command_key::command_action_key;
use crate::gate_engine::Opinion;
use crate::gate_flow::{RuleEntry, UserRules};
use crate::gate_full::StyleFlags;
use crate::machine_rules::inside_workdir;

pub(crate) fn style_hits(action: &GateAction, computer: &str, user: &UserRules) -> StyleFlags {
    let key = action.key();
    StyleFlags {
        ask_style_hit: user.ask_style.iter().any(|k| k == &key),
        allow_style_hit: user.allow_style.iter().any(|k| k == &key),
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

fn always_hit(action: &GateAction, computer: &str, always: &AlwaysList) -> bool {
    always_keys(action, computer)
        .iter()
        .any(|k| always.contains(k))
}

fn always_keys(action: &GateAction, computer: &str) -> Vec<String> {
    match &action.subject {
        ActionSubject::Command { line, .. } => {
            vec![format!(
                "command {} on {computer}",
                command_action_key(line)
            )]
        }
        ActionSubject::WriteFile { path, .. } if inside_workdir(path, &action.workdir) => {
            vec![format!("write workspace file on {computer}")]
        }
        ActionSubject::ReadFile { path }
        | ActionSubject::ListDir { path }
        | ActionSubject::WriteFile { path, .. }
        | ActionSubject::EditFile { path, .. }
        | ActionSubject::PathCheck { path } => {
            let effective = path.effective(&action.workdir);
            vec![
                format!("read file {effective} on {computer}"),
                format!("edit file {effective} on {computer}"),
            ]
        }
        ActionSubject::Capabilities => Vec::new(),
    }
}
