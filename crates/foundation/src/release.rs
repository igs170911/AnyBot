pub const STABLE_CHANNEL: &str = "stable";
pub const DEV_CHANNEL: &str = "dev";

pub const STABLE_APP_NAME: &str = "AnyBot";
pub const DEV_APP_NAME: &str = "AnyBot Dev";

pub const STABLE_APP_IDENTIFIER: &str = "app.anybot.desktop";
pub const DEV_APP_IDENTIFIER: &str = "app.anybot.desktop.dev";

#[must_use]
pub fn release_channel() -> &'static str {
    if release_is_dev() {
        DEV_CHANNEL
    } else {
        STABLE_CHANNEL
    }
}

#[must_use]
pub fn app_name() -> &'static str {
    if release_is_dev() {
        DEV_APP_NAME
    } else {
        STABLE_APP_NAME
    }
}

#[must_use]
pub fn app_identifier() -> &'static str {
    if release_is_dev() {
        DEV_APP_IDENTIFIER
    } else {
        STABLE_APP_IDENTIFIER
    }
}

#[must_use]
pub fn release_is_dev() -> bool {
    option_env!("ANYBOT_RELEASE_CHANNEL") == Some(DEV_CHANNEL)
}

#[must_use]
pub fn package_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[must_use]
pub fn version_string() -> String {
    version_line(
        package_version(),
        option_env!("ANYBOT_GIT_DESCRIBE").unwrap_or("unknown"),
        option_env!("SOURCE_DATE_EPOCH"),
    )
}

fn version_line(version: &str, describe: &str, epoch: Option<&str>) -> String {
    let mut commit = describe;
    let mut dirty = false;
    if let Some(stripped) = describe.strip_suffix("-dirty") {
        commit = stripped;
        dirty = true;
    }
    let short = if let Some((_, hash)) = commit.rsplit_once("-g") {
        hash
    } else if commit
        .strip_prefix('v')
        .is_some_and(|rest| rest.chars().next().is_some_and(|c| c.is_ascii_digit()))
    {
        "missing-commit"
    } else {
        commit
    };
    let mut line = format!("v{version}-{short}");
    if dirty {
        line.push_str("-dirty");
    }
    line.push_str(" · ");
    line.push_str(&build_time(epoch));
    line
}

fn build_time(epoch: Option<&str>) -> String {
    let Some(text) = epoch else {
        return "unspecified".to_owned();
    };
    let Ok(seconds) = text.parse::<i64>() else {
        return "unspecified".to_owned();
    };
    let Ok(utc) = time::OffsetDateTime::from_unix_timestamp(seconds) else {
        return "unspecified".to_owned();
    };
    match utc.format(&time::format_description::well_known::Rfc3339) {
        Ok(text) => text,
        Err(_) => "unspecified".to_owned(),
    }
}

#[must_use]
pub fn log_version(line: &str) -> &str {
    line.strip_prefix('v').unwrap_or(line)
}

#[cfg(test)]
#[path = "release_tests.rs"]
mod tests;
