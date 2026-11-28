use crate::action::{ActionSubject, PathView};
use crate::runner::GatedComputer;

pub(crate) fn resolve_subject(
    subject: ActionSubject,
    computer: &dyn GatedComputer,
    workdir: &str,
) -> ActionSubject {
    match subject {
        ActionSubject::ReadFile { path } => ActionSubject::ReadFile {
            path: resolve_one(&path, computer, workdir),
        },
        ActionSubject::WriteFile { path, content } => ActionSubject::WriteFile {
            path: resolve_one(&path, computer, workdir),
            content,
        },
        ActionSubject::EditFile {
            path,
            old_text,
            new_text,
        } => ActionSubject::EditFile {
            path: resolve_one(&path, computer, workdir),
            old_text,
            new_text,
        },
        ActionSubject::PathCheck { path } => ActionSubject::PathCheck {
            path: resolve_one(&path, computer, workdir),
        },
        ActionSubject::Command { .. } | ActionSubject::Capabilities => subject,
    }
}

pub(crate) fn resolve_one(
    view: &PathView,
    computer: &dyn GatedComputer,
    workdir: &str,
) -> PathView {
    if !view.is_resolvable() {
        return view.clone();
    }
    let raw = view.raw_text();
    let effective = if raw == "~" || raw.starts_with("~/") || raw.starts_with("~\\") {
        raw.to_owned()
    } else {
        view.effective(workdir)
    };
    match computer.resolve_target(&effective) {
        Some(canonical) => PathView::resolved(view.raw_text(), &canonical),
        None => PathView::unresolvable(view.raw_text()),
    }
}

pub(crate) fn resolved_targets(
    raw: &[PathView],
    computer: &dyn GatedComputer,
    workdir: &str,
) -> Vec<PathView> {
    raw.iter()
        .map(|view| resolve_one(view, computer, workdir))
        .collect()
}
