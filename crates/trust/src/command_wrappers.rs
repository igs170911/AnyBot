use crate::wrapper_flags::{
    COMMAND_FLAGS, DOAS_FLAGS, ENV_FLAGS, EXEC_FLAGS, Flag, NICE_FLAGS, STDBUF_FLAGS, SUDO_FLAGS,
    TIME_FLAGS, TIMEOUT_FLAGS, XARGS_FLAGS,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct WrapperSpec {
    name: &'static str,
    flags: &'static [(&'static str, Flag)],
    takes_duration: bool,
    stops: &'static [&'static str],
    cluster: bool,
}

const WRAPPERS: &[WrapperSpec] = &[
    WrapperSpec {
        name: "sudo",
        flags: SUDO_FLAGS,
        takes_duration: false,
        stops: &["-i", "--login", "-s", "--shell", "-e", "--edit"],
        cluster: true,
    },
    WrapperSpec {
        name: "doas",
        flags: DOAS_FLAGS,
        takes_duration: false,
        stops: &["-s", "-C", "-L"],
        cluster: true,
    },
    WrapperSpec {
        name: "env",
        flags: ENV_FLAGS,
        takes_duration: false,
        stops: &["-S"],
        cluster: true,
    },
    WrapperSpec {
        name: "nohup",
        flags: &[],
        takes_duration: false,
        stops: &[],
        cluster: false,
    },
    WrapperSpec {
        name: "nice",
        flags: NICE_FLAGS,
        takes_duration: false,
        stops: &[],
        cluster: false,
    },
    WrapperSpec {
        name: "timeout",
        flags: TIMEOUT_FLAGS,
        takes_duration: true,
        stops: &[],
        cluster: false,
    },
    WrapperSpec {
        name: "stdbuf",
        flags: STDBUF_FLAGS,
        takes_duration: false,
        stops: &[],
        cluster: false,
    },
    WrapperSpec {
        name: "time",
        flags: TIME_FLAGS,
        takes_duration: false,
        stops: &[],
        cluster: false,
    },
    WrapperSpec {
        name: "command",
        flags: COMMAND_FLAGS,
        takes_duration: false,
        stops: &["-v", "-V"],
        cluster: true,
    },
    WrapperSpec {
        name: "exec",
        flags: EXEC_FLAGS,
        takes_duration: false,
        stops: &[],
        cluster: true,
    },
    WrapperSpec {
        name: "xargs",
        flags: XARGS_FLAGS,
        takes_duration: false,
        stops: &["-I", "-J"],
        cluster: false,
    },
];

pub const PEEL_WRAPPERS: [&str; 11] = [
    "sudo", "doas", "env", "nohup", "nice", "timeout", "stdbuf", "time", "command", "exec", "xargs",
];

#[must_use]
pub fn wrapper_index(name: &str) -> Option<usize> {
    WRAPPERS.iter().position(|spec| spec.name == name)
}

#[must_use]
pub fn peel_known_wrapper(index: usize, words: &[String]) -> Option<Vec<String>> {
    let spec = WRAPPERS.get(index)?;
    let mut rest = words;
    let mut stopped = false;
    while let Some((head, tail)) = rest.split_first() {
        if head == "--" {
            rest = tail;
            break;
        }
        if let Some(flag) = flag_of(spec, head) {
            if spec.stops.contains(&flag) {
                stopped = true;
            }
            rest = skip_flag(spec, flag, tail)?;
            continue;
        }
        if spec.cluster && clustered(spec, head) {
            rest = tail;
            continue;
        }
        if is_assignment(head) {
            rest = tail;
            continue;
        }
        break;
    }
    if stopped {
        return None;
    }
    if spec.takes_duration {
        let (_, tail) = rest.split_first()?;
        rest = tail;
    }
    if rest.is_empty() {
        return None;
    }
    Some(rest.to_vec())
}

fn flag_of(spec: &WrapperSpec, word: &str) -> Option<&'static str> {
    if let Some((name, _)) = word.split_once('=') {
        let long = spec
            .flags
            .iter()
            .find(|(flag, _)| *flag == name && flag.starts_with("--"));
        return long.map(|(flag, _)| *flag);
    }
    spec.flags
        .iter()
        .find(|(flag, _)| *flag == word)
        .map(|(flag, _)| *flag)
}

fn skip_flag<'a>(
    spec: &WrapperSpec,
    flag: &'static str,
    tail: &'a [String],
) -> Option<&'a [String]> {
    let kind = spec.flags.iter().find(|(name, _)| *name == flag)?.1;
    match kind {
        Flag::Switch | Flag::OptionalValue => Some(tail),
        Flag::Value if flag.starts_with("--") => Some(tail),
        Flag::Value => tail.split_first().map(|(_, rest)| rest),
    }
}

fn clustered(spec: &WrapperSpec, word: &str) -> bool {
    let Some(body) = word.strip_prefix('-') else {
        return false;
    };
    if body.is_empty() || body.starts_with('-') || body.contains('=') {
        return false;
    }
    let switches: Vec<char> = spec
        .flags
        .iter()
        .filter(|(flag, kind)| *kind == Flag::Switch && flag.len() == 2)
        .filter_map(|(flag, _)| flag.chars().nth(1))
        .collect();
    body.chars().all(|ch| switches.contains(&ch))
}

fn is_assignment(word: &str) -> bool {
    let Some((name, _)) = word.split_once('=') else {
        return false;
    };
    let mut chars = name.chars();
    match chars.next() {
        Some(first) if first.is_ascii_alphabetic() || first == '_' => {}
        _ => return false,
    }
    chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}
