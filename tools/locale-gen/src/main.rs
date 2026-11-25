use std::io::Write as _;
use std::process::ExitCode;

use clap as _;
use clap::Parser;

use locale_gen::checks::{
    generated_freshness_violations, locale_pair_violations, usage_violations,
};
use locale_gen::cjk::{cjk_violations, is_excluded_path};
use locale_gen::codes::{rust_codes, ts_codes};
use locale_gen::parse::parse_ftl;
use locale_gen::runner::{
    RUST_CODES_REL, RunnerError, TS_CODES_REL, is_source_path, read_all, read_rel, repo_root,
    split_scan, tracked_files, write_rel,
};

#[derive(Parser)]
#[command(about = "Generate message code types from locales and check locale gates (spec/09 K91)")]
enum Args {
    Generate,
    Check,
}

fn main() -> ExitCode {
    match Args::parse() {
        Args::Generate => to_code(generate()),
        Args::Check => to_code(check()),
    }
}

fn to_code(result: Result<(), Vec<String>>) -> ExitCode {
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(violations) => {
            for v in violations {
                report(&format!("locale-gen: {v}"));
            }
            ExitCode::FAILURE
        }
    }
}

fn generate() -> Result<(), Vec<String>> {
    let root = repo_root().map_err(|e| io_failure(&e))?;
    let en = read_rel(&root, "locales/en.ftl").map_err(|e| io_failure(&e))?;
    let entries = parse_ftl(&en);
    write_rel(
        &root,
        RUST_CODES_REL,
        &rust_codes(&entries).map_err(code_failure)?,
    )
    .map_err(|e| io_failure(&e))?;
    write_rel(&root, TS_CODES_REL, &ts_codes(&entries)).map_err(|e| io_failure(&e))?;
    report(&format!(
        "locale-gen: wrote generated code types for {} codes",
        entries.len()
    ));
    Ok(())
}

fn report(message: &str) {
    let mut stderr = std::io::stderr();
    let _result = writeln!(stderr, "{message}");
}

fn io_failure(e: &RunnerError) -> Vec<String> {
    vec![e.to_string()]
}

fn code_failure(e: String) -> Vec<String> {
    vec![e]
}

fn check() -> Result<(), Vec<String>> {
    let root = repo_root().map_err(|e| io_failure(&e))?;
    let paths = tracked_files(&root).map_err(|e| io_failure(&e))?;
    let files = read_all(&root, &paths).map_err(|e| io_failure(&e))?;
    let mut violations = locale_violations(&root, &files)?;
    let scan_files: Vec<(String, String)> = files
        .into_iter()
        .filter(|(path, _)| !is_excluded_path(path))
        .collect();
    violations.extend(cjk_violations(&scan_files));
    if violations.is_empty() {
        report(&format!(
            "locale-gen: locale gates OK ({} tracked files scanned)",
            scan_files.len()
        ));
        return Ok(());
    }
    Err(violations)
}

fn locale_violations(
    root: &std::path::Path,
    files: &[(String, String)],
) -> Result<Vec<String>, Vec<String>> {
    let en = read_rel(root, "locales/en.ftl").map_err(|e| io_failure(&e))?;
    let zh = read_rel(root, "locales/zh-TW.ftl").map_err(|e| io_failure(&e))?;
    let committed_rust = read_rel(root, RUST_CODES_REL).map_err(|e| io_failure(&e))?;
    let committed_ts = read_rel(root, TS_CODES_REL).map_err(|e| io_failure(&e))?;
    let mut violations = locale_pair_violations(&en, &zh);
    let entries = parse_ftl(&en);
    violations.extend(generated_freshness_violations(
        &entries,
        &committed_rust,
        &committed_ts,
        &rust_codes(&entries).map_err(code_failure)?,
        &ts_codes(&entries),
    ));
    let sources: Vec<(String, String)> = files
        .iter()
        .filter(|(path, _)| is_source_path(path))
        .cloned()
        .collect();
    let (rust_scan, ts_scan, all_scan) = split_scan(&sources);
    violations.extend(usage_violations(&en, &rust_scan, &ts_scan, &all_scan));
    Ok(violations)
}
