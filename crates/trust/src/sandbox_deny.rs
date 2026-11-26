pub const DENIED_COMMAND_KINDS: [&str; 10] = [
    "sudo",
    "su",
    "rm -rf /",
    "rm -rf ~",
    "mkfs",
    "dd",
    "shutdown",
    "reboot",
    "diskutil erase",
    "chmod -R 777 /",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DeniedShape {
    Name(&'static str),
    NamePrefix(&'static str),
    RmRoot,
    RmHome,
    DiskutilErase,
    ChmodRoot,
}

const JOIN_TOKENS: [&str; 6] = [";", "&&", "||", "|", "&", "\n"];

#[must_use]
pub fn normalize_command(text: &str) -> String {
    let lowered = text.to_lowercase();
    let mut out = String::with_capacity(lowered.len());
    let mut space = false;
    for c in lowered.chars() {
        if c.is_whitespace() {
            space = true;
            continue;
        }
        if space && !out.is_empty() {
            out.push(' ');
        }
        space = false;
        out.push(c);
    }
    out
}

#[must_use]
pub fn split_segments(line: &str) -> Vec<String> {
    let bytes = line.as_bytes();
    let mut raw = Vec::new();
    let mut start = 0usize;
    let mut pos = 0usize;
    while pos < bytes.len() {
        let Some(len) = join_token_len_at(bytes, pos) else {
            pos = pos.saturating_add(1);
            continue;
        };
        raw.push(bytes.get(start..pos).unwrap_or(&[]).to_vec());
        pos = pos.saturating_add(len);
        start = pos;
    }
    raw.push(bytes.get(start..).unwrap_or(&[]).to_vec());
    raw.into_iter()
        .map(|chunk| normalize_command(&String::from_utf8_lossy(&chunk)))
        .filter(|s| !s.is_empty())
        .collect()
}

fn join_token_len_at(bytes: &[u8], pos: usize) -> Option<usize> {
    JOIN_TOKENS
        .iter()
        .filter(|t| starts_with_at(bytes, pos, t.as_bytes()))
        .map(|t| t.len())
        .max()
}

fn starts_with_at(bytes: &[u8], pos: usize, token: &[u8]) -> bool {
    let Some(window) = bytes.get(pos..pos.saturating_add(token.len())) else {
        return false;
    };
    window == token
}

#[must_use]
pub fn deny_token(line: &str, bot_machine: bool) -> Option<&'static str> {
    let crate::command_read::ReadLine::Read(commands) = crate::command_read::read_line(line) else {
        return None;
    };
    commands
        .iter()
        .find_map(|command| denied_kind(command, bot_machine))
}

fn denied_kind(
    command: &crate::command_read::ReadCommand,
    bot_machine: bool,
) -> Option<&'static str> {
    let name = command.name.as_str();
    if !bot_machine && (matches!(name, "sudo" | "doas") || command.peeled_privilege) {
        return Some("sudo");
    }
    match denied_shape(name, &command.args) {
        Some(DeniedShape::Name(kind) | DeniedShape::NamePrefix(kind)) => Some(kind),
        Some(DeniedShape::RmRoot) => Some("rm -rf /"),
        Some(DeniedShape::RmHome) => Some("rm -rf ~"),
        Some(DeniedShape::DiskutilErase) => Some("diskutil erase"),
        Some(DeniedShape::ChmodRoot) => Some("chmod -R 777 /"),
        None => None,
    }
}

fn denied_shape(name: &str, args: &[String]) -> Option<DeniedShape> {
    if matches!(name, "su" | "dd" | "shutdown" | "reboot") {
        return Some(DeniedShape::Name(name_kind(name)));
    }
    if name == "mkfs" || name.starts_with("mkfs.") {
        return Some(DeniedShape::NamePrefix("mkfs"));
    }
    if name == "rm" && removes_root(args) {
        return Some(if home_target(args) {
            DeniedShape::RmHome
        } else {
            DeniedShape::RmRoot
        });
    }
    if name == "diskutil"
        && args
            .first()
            .is_some_and(|arg| arg == "erase" || arg.starts_with("erase"))
    {
        return Some(DeniedShape::DiskutilErase);
    }
    if name == "chmod" && chmod_root(args) {
        return Some(DeniedShape::ChmodRoot);
    }
    None
}

fn name_kind(name: &str) -> &'static str {
    match name {
        "su" => "su",
        "dd" => "dd",
        "shutdown" => "shutdown",
        _ => "reboot",
    }
}

fn removes_root(args: &[String]) -> bool {
    let mut recursive = false;
    let mut force = false;
    let mut targets = Vec::new();
    for arg in args {
        if let Some(flags) = arg.strip_prefix('-') {
            if flags.chars().any(|ch| ch == 'r' || ch == 'R') {
                recursive = true;
            }
            if flags.contains('f') {
                force = true;
            }
            continue;
        }
        targets.push(arg.as_str());
    }
    recursive
        && force
        && targets
            .iter()
            .any(|target| is_root(target) || is_home(target))
}

fn home_target(args: &[String]) -> bool {
    args.iter().any(|arg| !arg.starts_with('-') && is_home(arg))
}

fn is_root(target: &str) -> bool {
    target == "/" || target == "/*"
}

fn is_home(target: &str) -> bool {
    target == "~" || target == "~/" || target.starts_with("~/")
}

fn chmod_root(args: &[String]) -> bool {
    let mut recursive = false;
    let mut mode = String::new();
    let mut targets = Vec::new();
    for arg in args {
        if arg == "-r" || arg == "-R" || arg == "--recursive" {
            recursive = true;
            continue;
        }
        if arg.starts_with('-') {
            continue;
        }
        if mode.is_empty() {
            mode.clone_from(arg);
            continue;
        }
        targets.push(arg.as_str());
    }
    recursive && mode == "777" && targets.iter().any(|target| is_root(target))
}
