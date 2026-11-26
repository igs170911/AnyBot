#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Flag {
    Switch,
    Value,
    OptionalValue,
}

pub(crate) const SUDO_FLAGS: &[(&str, Flag)] = &[
    ("-A", Flag::Switch),
    ("--askpass", Flag::Switch),
    ("-B", Flag::Switch),
    ("--bell", Flag::Switch),
    ("-b", Flag::Switch),
    ("--background", Flag::Switch),
    ("-C", Flag::Value),
    ("--close-from", Flag::Value),
    ("-D", Flag::Value),
    ("--chdir", Flag::Value),
    ("-E", Flag::Switch),
    ("--preserve-env", Flag::OptionalValue),
    ("-e", Flag::Switch),
    ("--edit", Flag::Switch),
    ("-g", Flag::Value),
    ("--group", Flag::Value),
    ("-H", Flag::Switch),
    ("--set-home", Flag::Switch),
    ("-h", Flag::Value),
    ("--help", Flag::Switch),
    ("--host", Flag::Value),
    ("-i", Flag::Switch),
    ("--login", Flag::Switch),
    ("-K", Flag::Switch),
    ("--remove-timestamp", Flag::Switch),
    ("-k", Flag::Switch),
    ("--reset-timestamp", Flag::Switch),
    ("-l", Flag::Switch),
    ("--list", Flag::Switch),
    ("-N", Flag::Switch),
    ("--no-update", Flag::Switch),
    ("-n", Flag::Switch),
    ("--non-interactive", Flag::Switch),
    ("-P", Flag::Switch),
    ("--preserve-groups", Flag::Switch),
    ("-p", Flag::Value),
    ("--prompt", Flag::Value),
    ("-R", Flag::Value),
    ("--chroot", Flag::Value),
    ("-S", Flag::Switch),
    ("--stdin", Flag::Switch),
    ("-s", Flag::Switch),
    ("--shell", Flag::Switch),
    ("-U", Flag::Value),
    ("--other-user", Flag::Value),
    ("-T", Flag::Value),
    ("--command-timeout", Flag::Value),
    ("-u", Flag::Value),
    ("--user", Flag::Value),
    ("-V", Flag::Switch),
    ("--version", Flag::Switch),
    ("-v", Flag::Switch),
    ("--validate", Flag::Switch),
];

pub(crate) const DOAS_FLAGS: &[(&str, Flag)] = &[
    ("-L", Flag::Switch),
    ("-n", Flag::Switch),
    ("-s", Flag::Switch),
    ("-a", Flag::Value),
    ("-C", Flag::Value),
    ("-u", Flag::Value),
];

pub(crate) const ENV_FLAGS: &[(&str, Flag)] = &[
    ("-0", Flag::Switch),
    ("-i", Flag::Switch),
    ("-v", Flag::Switch),
    ("-", Flag::Switch),
    ("-u", Flag::Value),
    ("-C", Flag::Value),
    ("-P", Flag::Value),
    ("-S", Flag::Value),
];

pub(crate) const NICE_FLAGS: &[(&str, Flag)] = &[("-n", Flag::Value)];

pub(crate) const TIMEOUT_FLAGS: &[(&str, Flag)] = &[
    ("-f", Flag::Switch),
    ("--foreground", Flag::Switch),
    ("-p", Flag::Switch),
    ("--preserve-status", Flag::Switch),
    ("-v", Flag::Switch),
    ("--verbose", Flag::Switch),
    ("-k", Flag::Value),
    ("--kill-after", Flag::Value),
    ("-s", Flag::Value),
    ("--signal", Flag::Value),
];

pub(crate) const STDBUF_FLAGS: &[(&str, Flag)] = &[
    ("-e", Flag::Value),
    ("-i", Flag::Value),
    ("-o", Flag::Value),
];

pub(crate) const TIME_FLAGS: &[(&str, Flag)] = &[
    ("-a", Flag::Switch),
    ("-h", Flag::Switch),
    ("-l", Flag::Switch),
    ("-p", Flag::Switch),
    ("-o", Flag::Value),
];

pub(crate) const COMMAND_FLAGS: &[(&str, Flag)] = &[
    ("-p", Flag::Switch),
    ("-V", Flag::Switch),
    ("-v", Flag::Switch),
];

pub(crate) const EXEC_FLAGS: &[(&str, Flag)] = &[
    ("-c", Flag::Switch),
    ("-l", Flag::Switch),
    ("-a", Flag::Value),
];

pub(crate) const XARGS_FLAGS: &[(&str, Flag)] = &[
    ("-0", Flag::Switch),
    ("--null", Flag::Switch),
    ("-o", Flag::Switch),
    ("-p", Flag::Switch),
    ("--interactive", Flag::Switch),
    ("-r", Flag::Switch),
    ("--no-run-if-empty", Flag::Switch),
    ("-t", Flag::Switch),
    ("--verbose", Flag::Switch),
    ("-x", Flag::Switch),
    ("--exit", Flag::Switch),
    ("-E", Flag::Value),
    ("-I", Flag::Value),
    ("-J", Flag::Value),
    ("-L", Flag::Value),
    ("-n", Flag::Value),
    ("--max-args", Flag::Value),
    ("-P", Flag::Value),
    ("--max-procs", Flag::Value),
    ("-R", Flag::Value),
    ("-S", Flag::Value),
    ("-s", Flag::Value),
    ("--max-chars", Flag::Value),
];
