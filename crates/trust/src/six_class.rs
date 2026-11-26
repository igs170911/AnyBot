use crate::action::{ActionSubject, GateAction, PathView};
use crate::machine_rules::inside_workdir;
use crate::safety_n5::{ActionClass, classes_of_command};

#[must_use]
pub(crate) fn holds_a_class(action: &GateAction, caps_bot: bool) -> bool {
    match &action.subject {
        ActionSubject::Command { line, .. } => command_holds(action, line, caps_bot),
        ActionSubject::WriteFile { path, .. } | ActionSubject::EditFile { path, .. } => {
            write_holds(path, &action.workdir)
        }
        ActionSubject::ReadFile { .. }
        | ActionSubject::PathCheck { .. }
        | ActionSubject::Capabilities => false,
    }
}

fn command_holds(action: &GateAction, line: &str, caps_bot: bool) -> bool {
    let classes = classes_of_command(line);
    if classes.iter().copied().any(outbound) {
        return true;
    }
    if !classes.contains(&ActionClass::Delete) {
        return false;
    }
    if !caps_bot {
        return true;
    }
    !own_folder_delete(action)
}

fn outbound(class: ActionClass) -> bool {
    matches!(
        class,
        ActionClass::SendToOthers
            | ActionClass::Share
            | ActionClass::Publish
            | ActionClass::Payment
            | ActionClass::PasswordChange
    )
}

fn own_folder_delete(action: &GateAction) -> bool {
    let targets = action.path_views();
    !targets.is_empty()
        && targets
            .iter()
            .all(|path| inside_workdir(path, &action.workdir))
}

fn write_holds(path: &PathView, workdir: &str) -> bool {
    !inside_workdir(path, workdir)
}
