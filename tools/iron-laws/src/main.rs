#![forbid(unsafe_code)]

use serde_json as _;
use sqlparser as _;
use yaml_serde as _;

use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

fn repo_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    match manifest.join("../..").canonicalize() {
        Ok(p) => p,
        Err(_) => manifest,
    }
}

fn main() -> ExitCode {
    let git_ref = std::env::args().nth(1).unwrap_or_default();
    if git_ref.is_empty() {
        let mut stdout = std::io::stdout();
        if writeln!(
            stdout,
            "iron-laws: no comparison base (SPEC_TRACE_REF must be the merge base; there is no HEAD fallback)"
        )
        .is_err()
        {
            return ExitCode::FAILURE;
        }
        return ExitCode::FAILURE;
    }
    let violations = iron_laws::run(&repo_root(), &git_ref);
    let mut stdout = std::io::stdout();
    for v in &violations {
        if writeln!(stdout, "{v}").is_err() {
            return ExitCode::FAILURE;
        }
    }
    if violations.is_empty() {
        if writeln!(stdout, "iron-laws: all gates pass").is_err() {
            return ExitCode::FAILURE;
        }
        return ExitCode::SUCCESS;
    }
    let tail = format!(
        "iron-laws: {} violations (fix the code until they pass; do not silence the checks)",
        violations.len()
    );
    if writeln!(stdout, "{tail}").is_err() {
        return ExitCode::FAILURE;
    }
    ExitCode::FAILURE
}
