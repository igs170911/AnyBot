pub use crate::command_read::{
    CODE_TOOL_UNREADABLE_SHAPE as CODE_TOOL_SHAPE, SHELL_HEADS, bare_name, command_text, read_line,
};
pub use crate::command_wrappers::PEEL_WRAPPERS;

pub const PRIVILEGE_WRAPPERS: [&str; 2] = ["sudo", "doas"];

pub const CODE_TOOL_EVAL: &str = CODE_TOOL_SHAPE;
pub const CODE_TOOL_UNBALANCED: &str = CODE_TOOL_SHAPE;
pub const CODE_TOOL_HEREDOC: &str = CODE_TOOL_SHAPE;
pub const CODE_TOOL_HEAD: &str = CODE_TOOL_SHAPE;

#[must_use]
pub fn visible_segments(line: &str) -> Vec<String> {
    match read_line(line) {
        crate::command_read::ReadLine::Read(commands) => {
            commands.iter().map(command_text).collect()
        }
        crate::command_read::ReadLine::Opaque(_) => Vec::new(),
    }
}

#[must_use]
pub fn peel_segment(segment: &str, bot_machine: bool) -> String {
    let kept = bot_machine;
    if kept {
        return peeled(segment);
    }
    peeled(segment)
}

fn peeled(segment: &str) -> String {
    match read_line(segment) {
        crate::command_read::ReadLine::Read(commands) => match commands.first() {
            Some(command) => command_text(command),
            None => segment.to_owned(),
        },
        crate::command_read::ReadLine::Opaque(_) => segment.to_owned(),
    }
}

#[must_use]
pub fn unreadable_reason(line: &str) -> Option<&'static str> {
    match read_line(line) {
        crate::command_read::ReadLine::Opaque(code) => Some(code),
        crate::command_read::ReadLine::Read(_) => None,
    }
}
