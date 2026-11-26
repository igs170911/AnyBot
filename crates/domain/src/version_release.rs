use jiff::Span;
use jiff::Timestamp;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseRow {
    pub version: String,
    pub at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseTable {
    pub rows: Vec<ReleaseRow>,
}

impl ReleaseTable {
    #[must_use]
    pub fn empty() -> Self {
        ReleaseTable { rows: Vec::new() }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VersionVerdict {
    Accept,
    TooOld,
    Unreadable,
}

#[must_use]
pub fn judge_client_version(
    header: Option<&str>,
    table: &ReleaseTable,
    now_ms: i64,
) -> VersionVerdict {
    let Some(text) = header else {
        return VersionVerdict::Unreadable;
    };
    let Some(core) = core_version(text) else {
        return VersionVerdict::Unreadable;
    };
    let Some(latest) = latest_release(table) else {
        return VersionVerdict::Accept;
    };
    if core >= latest {
        return VersionVerdict::Accept;
    }
    let Some(replacement) = first_release_after(table, &core) else {
        return VersionVerdict::Accept;
    };
    if now_ms <= six_months_after(replacement.at_ms) {
        return VersionVerdict::Accept;
    }
    VersionVerdict::TooOld
}

fn core_version(text: &str) -> Option<String> {
    let head = text.split('-').next().unwrap_or(text);
    let parts: Vec<&str> = head.split('.').collect();
    if parts.len() != 3 {
        return None;
    }
    let numbers: Vec<u64> = parts
        .iter()
        .map(|p| p.parse::<u64>().ok())
        .collect::<Option<Vec<u64>>>()?;
    if numbers.iter().any(|n| *n > u64::from(u32::MAX)) {
        return None;
    }
    Some(
        numbers
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<String>>()
            .join("."),
    )
}

fn latest_release(table: &ReleaseTable) -> Option<String> {
    table
        .rows
        .iter()
        .map(|row| row.version.clone())
        .max_by(|a, b| compare_versions(a, b))
}

fn first_release_after<'t>(table: &'t ReleaseTable, version: &str) -> Option<&'t ReleaseRow> {
    table
        .rows
        .iter()
        .filter(|row| compare_versions(&row.version, version) == std::cmp::Ordering::Greater)
        .min_by(|a, b| compare_versions(&a.version, &b.version))
}

fn compare_versions(a: &str, b: &str) -> std::cmp::Ordering {
    let Some(x) = semver::Version::parse(a).ok() else {
        return std::cmp::Ordering::Equal;
    };
    let Some(y) = semver::Version::parse(b).ok() else {
        return std::cmp::Ordering::Equal;
    };
    x.cmp(&y)
}

#[must_use]
pub fn six_months_after(at_ms: i64) -> i64 {
    let Some(start) = Timestamp::from_millisecond(at_ms).ok() else {
        return at_ms;
    };
    let Some(local) = start.in_tz("UTC").ok().map(|value| value.datetime()) else {
        return at_ms;
    };
    let span = Span::new().months(6);
    let Some(shifted) = local.checked_add(span).ok() else {
        return at_ms;
    };
    let Some(zoned) = shifted.in_tz("UTC").ok() else {
        return at_ms;
    };
    zoned.timestamp().as_millisecond()
}
