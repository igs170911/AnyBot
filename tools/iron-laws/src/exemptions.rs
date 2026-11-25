use std::collections::{BTreeMap, BTreeSet};

use crate::table::{CMD_T, FS_M, FS_STD_M, FS_T, HTTP_M, PROC_M};

#[must_use]
pub fn clippy_exemptions() -> BTreeMap<String, BTreeMap<String, BTreeSet<String>>> {
    let mut m = BTreeMap::new();
    insert(
        &mut m,
        "anybot-foundation",
        &[
            "std::fs::read_to_string",
            "std::fs::File::open",
            "std::env::home_dir",
        ],
        &["std::fs::File"],
    );
    insert(
        &mut m,
        "anybot-computers",
        &merge(&[&PROC_M, &HTTP_M, &FS_M]),
        &[CMD_T[0], CMD_T[1], FS_T[0], FS_T[1], FS_T[2], FS_T[3]],
    );
    insert(&mut m, "anybot-models", &HTTP_M, &[]);
    insert(&mut m, "anybot-mcp", &HTTP_M, &[]);
    insert(&mut m, "anybot-sync", &HTTP_M, &[]);
    insert(&mut m, "anybot-channels", &HTTP_M, &[]);
    insert(&mut m, "anybot-blob", &FS_M, &FS_T);
    insert(&mut m, "anybot-store-client", &FS_M, &FS_T);
    insert(
        &mut m,
        "depcheck",
        &merge(&[&STD_PROC, &FS_STD_M]),
        &[CMD_T[0], FS_T[0], FS_T[1], FS_T[2]],
    );
    insert(
        &mut m,
        "spec-trace",
        &merge(&[&STD_PROC, &FS_STD_M]),
        &[CMD_T[0], FS_T[0], FS_T[1], FS_T[2]],
    );
    insert(
        &mut m,
        "iron-laws",
        &merge(&[&STD_PROC, &FS_STD_M]),
        &[CMD_T[0], FS_T[0], FS_T[1], FS_T[2]],
    );
    insert(&mut m, "api-docs", &FS_STD_M, &[FS_T[0], FS_T[1], FS_T[2]]);
    insert(
        &mut m,
        "test-harness",
        &merge(&[&PROC_M, &HTTP_M, &FS_M]),
        &[CMD_T[0], CMD_T[1], FS_T[0], FS_T[1], FS_T[2], FS_T[3]],
    );
    m
}

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
