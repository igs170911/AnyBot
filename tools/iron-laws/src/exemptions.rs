use std::collections::{BTreeMap, BTreeSet};

use crate::table::{CMD_T, FS_M, FS_STD_M, FS_T, HTTP_M, PROC_M, TIME_M};

#[must_use]
pub fn clippy_exemptions() -> BTreeMap<String, BTreeMap<String, BTreeSet<String>>> {
    let mut m = BTreeMap::new();
    fill_server_packages(&mut m);
    fill_client_packages(&mut m);
    fill_tool_packages(&mut m);
    m
}

fn fill_server_packages(m: &mut BTreeMap<String, BTreeMap<String, BTreeSet<String>>>) {
    insert(
        m,
        "anybot-foundation",
        &[
            "std::fs::read_to_string",
            "std::fs::File::open",
            "std::fs::read_dir",
            "std::env::home_dir",
            "time::OffsetDateTime::now_utc",
            "tokio::time::sleep",
            "tokio::time::sleep_until",
            "tokio::time::timeout",
            "tokio::time::Instant::now",
        ],
        &["std::fs::File"],
    );
    insert(
        m,
        "anybot-computers",
        &merge(&[&PROC_M, &HTTP_M, &FS_M]),
        &[CMD_T[0], CMD_T[1], FS_T[0], FS_T[1], FS_T[2], FS_T[3]],
    );
    insert(m, "anybot-models", &merge(&[&HTTP_M, &PERMIT_M]), &[]);
    insert(m, "anybot-mcp", &HTTP_M, &[]);
    insert(m, "anybot-sync", &HTTP_M, &[]);
    insert(m, "anybot-channels", &HTTP_M, &[]);
    insert(m, "anybot-blob", &FS_M, &FS_T);
    insert(m, "anybot-store-client", &FS_M, &FS_T);
}

fn fill_client_packages(m: &mut BTreeMap<String, BTreeMap<String, BTreeSet<String>>>) {
    insert(
        m,
        "test-harness",
        &merge(&[&PROC_M, &HTTP_M, &FS_M, &TIME_M, &PERMIT_M]),
        &[CMD_T[0], CMD_T[1], FS_T[0], FS_T[1], FS_T[2], FS_T[3]],
    );
    insert(m, "live-model", &merge(&[&HTTP_M, &FS_M, &PERMIT_M]), &FS_T);
    insert(
        m,
        "os-certs",
        &merge(&[&FS_M, &STD_PROC]),
        &[CMD_T[0], FS_T[0], FS_T[1], FS_T[2]],
    );
}

fn fill_tool_packages(m: &mut BTreeMap<String, BTreeMap<String, BTreeSet<String>>>) {
    let tool_methods = merge(&[&STD_PROC, &FS_STD_M]);
    let tool_types = &[CMD_T[0], FS_T[0], FS_T[1], FS_T[2]];
    insert(m, "depcheck", &tool_methods, tool_types);
    insert(m, "spec-trace", &tool_methods, tool_types);
    insert(m, "iron-laws", &tool_methods, tool_types);
    insert(m, "locale-gen", &tool_methods, tool_types);
    insert(m, "api-docs", &FS_STD_M, &[FS_T[0], FS_T[1], FS_T[2]]);
}

pub(crate) const PERMIT_M: [&str; 2] = [
    "anybot_models::SendPermit::for_endpoint",
    "anybot_models::Gateway::pin_host",
];

const STD_PROC: [&str; 4] = [
    "std::process::Command::new",
    "std::process::Command::output",
    "std::process::Command::spawn",
    "std::process::Command::status",
];

#[must_use]
pub fn merge<'a>(parts: &'a [&'a [&'a str]]) -> Vec<&'a str> {
    parts.iter().flat_map(|p| p.iter().copied()).collect()
}

fn insert(
    m: &mut BTreeMap<String, BTreeMap<String, BTreeSet<String>>>,
    pkg: &str,
    methods: &[&str],
    types: &[&str],
) {
    let mut per_key = BTreeMap::new();
    if !methods.is_empty() {
        per_key.insert(
            "disallowed-methods".to_owned(),
            methods
                .iter()
                .map(|p| (*p).to_owned())
                .collect::<BTreeSet<_>>(),
        );
    }
    if !types.is_empty() {
        per_key.insert(
            "disallowed-types".to_owned(),
            types
                .iter()
                .map(|p| (*p).to_owned())
                .collect::<BTreeSet<_>>(),
        );
    }
    m.insert(pkg.to_owned(), per_key);
}
