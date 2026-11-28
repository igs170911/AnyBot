use std::collections::BTreeMap;

use anybot_protocol::EnvPlan;

pub const MODEL_KEYS: [&str; 6] = [
    "XAI_API_KEY",
    "OPENAI_API_KEY",
    "ANTHROPIC_API_KEY",
    "GEMINI_API_KEY",
    "OPENROUTER_API_KEY",
    "ANTHROPIC_AUTH_TOKEN",
];

pub const ACCOUNT_KEYS: [&str; 1] = ["ANYBOT_ACCOUNT_PASSWORD"];

pub const RISKY_EXACT: [&str; 5] = [
    "LD_PRELOAD",
    "LD_LIBRARY_PATH",
    "BASH_ENV",
    "ENV",
    "PROMPT_COMMAND",
];

pub const RISKY_PREFIX: [&str; 1] = ["DYLD_"];

pub const COMMAND_ONLY: [&str; 2] = ["SSH_AUTH_SOCK", "DOCKER_HOST"];

#[must_use]
pub fn command_env_plan() -> EnvPlan {
    let mut unset_exact = Vec::new();
    for key in MODEL_KEYS
        .iter()
        .chain(ACCOUNT_KEYS.iter())
        .chain(RISKY_EXACT.iter())
        .chain(COMMAND_ONLY.iter())
    {
        unset_exact.push((*key).to_owned());
    }
    let unset_prefix = RISKY_PREFIX.iter().map(|p| (*p).to_owned()).collect();
    EnvPlan {
        unset_exact,
        unset_prefix,
    }
}

#[must_use]
pub fn apply_env_plan(inherited: &[(String, String)], plan: &EnvPlan) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for (key, value) in inherited {
        if should_drop(key, plan) {
            continue;
        }
        out.insert(key.clone(), value.clone());
    }
    out
}

fn should_drop(key: &str, plan: &EnvPlan) -> bool {
    plan.unset_exact.iter().any(|name| name == key)
        || plan
            .unset_prefix
            .iter()
            .any(|prefix| key.starts_with(prefix))
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXPECTED_EXACT: [&str; 14] = [
        "XAI_API_KEY",
        "OPENAI_API_KEY",
        "ANTHROPIC_API_KEY",
        "GEMINI_API_KEY",
        "OPENROUTER_API_KEY",
        "ANTHROPIC_AUTH_TOKEN",
        "ANYBOT_ACCOUNT_PASSWORD",
        "LD_PRELOAD",
        "LD_LIBRARY_PATH",
        "BASH_ENV",
        "ENV",
        "PROMPT_COMMAND",
        "SSH_AUTH_SOCK",
        "DOCKER_HOST",
    ];

    #[test]
    fn env_plan_lists_every_key() {
        let plan = command_env_plan();
        for key in EXPECTED_EXACT {
            assert!(
                plan.unset_exact.iter().any(|item| item == key),
                "missing exact key {key}"
            );
        }
        assert_eq!(plan.unset_exact.len(), EXPECTED_EXACT.len());
        assert_eq!(plan.unset_prefix, vec!["DYLD_".to_owned()]);
    }

    #[test]
    fn env_plan_keeps_ordinary_vars() {
        let mut inherited: Vec<(String, String)> = EXPECTED_EXACT
            .iter()
            .map(|k| ((*k).to_owned(), "secret".to_owned()))
            .collect();
        inherited.push(("DYLD_INSERT_LIBRARIES".to_owned(), "/bad.dylib".to_owned()));
        inherited.push(("DYLD_LIBRARY_PATH".to_owned(), "/bad/lib".to_owned()));
        inherited.push(("PATH".to_owned(), "/usr/bin".to_owned()));
        inherited.push(("HOME".to_owned(), "/home/alice".to_owned()));
        let kept = apply_env_plan(&inherited, &command_env_plan());
        assert_eq!(kept.get("PATH").map(String::as_str), Some("/usr/bin"));
        assert_eq!(kept.get("HOME").map(String::as_str), Some("/home/alice"));
        assert_eq!(kept.len(), 2);
    }
}
