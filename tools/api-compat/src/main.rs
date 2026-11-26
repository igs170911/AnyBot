use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;

use api_compat::{compat, schema};

use clap as _;
use serde_json as _;
use sha2 as _;
use yaml_serde as _;

use serde as _;

#[derive(Parser)]
#[command(
    name = "api-compat",
    version,
    about = "compare the committed API and protocol baselines with the comparison ref (spec/09 K118 6)"
)]
struct Args {
    #[arg(long, default_value = "HEAD")]
    against: String,
    #[arg(long)]
    repo: Option<PathBuf>,
}

fn repo_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    match manifest.join("../..").canonicalize() {
        Ok(p) => p,
        Err(_) => manifest,
    }
}

fn main() -> ExitCode {
    let args = Args::parse();
    let repo = args.repo.unwrap_or_else(repo_root);
    let inputs = compat::Inputs {
        repo: &repo,
        git_ref: &args.against,
        read_ref: &compat::read_ref,
        tracked: &compat::changelog_tracked,
        release_tags: &compat::release_tags,
        breaking_of: &schema::breaking,
    };
    let (violations, notes) = compat::violations_with_notes(&inputs);
    let mut stdout = std::io::stdout();
    for note in &notes {
        if writeln!(stdout, "info: {note}").is_err() {
            return ExitCode::FAILURE;
        }
    }
    for v in &violations {
        if writeln!(stdout, "{v}").is_err() {
            return ExitCode::FAILURE;
        }
    }
    if violations.is_empty() {
        if writeln!(
            stdout,
            "api-compat: baselines and the breaking-change log are consistent"
        )
        .is_err()
        {
            return ExitCode::FAILURE;
        }
        return ExitCode::SUCCESS;
    }
    ExitCode::FAILURE
}
