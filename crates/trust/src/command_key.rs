use std::borrow::ToOwned;

const TWO_TOKEN_COMMANDS: [&str; 11] = [
    "git", "npm", "pnpm", "yarn", "cargo", "brew", "pip", "pip3", "python3", "docker", "go",
];

#[must_use]
pub fn command_tokens(command: &str) -> Vec<String> {
    command
        .split_whitespace()
        .take(2)
        .map(ToOwned::to_owned)
        .collect()
}

#[must_use]
pub fn command_action_key(command: &str) -> String {
    let tokens = command_tokens(command);
    let Some(first) = tokens.first() else {
        return String::new();
    };
    let second = tokens.get(1).map(String::as_str).unwrap_or_default();
    let known = TWO_TOKEN_COMMANDS.iter().any(|c| c == first);
    if known && !second.is_empty() {
        return format!("{first} {second}");
    }
    first.clone()
}
