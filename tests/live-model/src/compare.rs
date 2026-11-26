use std::io::Write;
use std::path::{Path, PathBuf};

use test_harness::repo_root;

use crate::profile::{LiveProfile, load_profile};
use crate::side::{Shapes, fake_side, live_side, report_lines, write_report};

pub async fn run() -> Result<(), String> {
    let repo = repo_root();
    let profile = load_profile(&repo)?;
    let outcome = run_with_profile(&repo, &profile).await;
    emit(&outcome)?;
    outcome.failure()
}

pub struct Outcome {
    pub shapes: Shapes,
    pub product: Vec<String>,
    pub report: Result<PathBuf, String>,
    pub failures: Vec<String>,
}

async fn run_with_profile(repo: &Path, profile: &LiveProfile) -> Outcome {
    let fake = fake_side(&profile.model, profile.always_thinks).await;
    let live = live_side(profile).await;
    let shapes = Shapes {
        fake: fake.side.probe,
        live: live.side.probe,
    };
    let product = crate::product::product_lines(&fake.side.product, &live.side.product);
    let report = write_report(repo, &shapes, &product);
    let failures = collect_failures(fake.failed, live.failed, &report);
    Outcome {
        shapes,
        product,
        report,
        failures,
    }
}

fn collect_failures(
    fake: Option<String>,
    live: Option<String>,
    report: &Result<PathBuf, String>,
) -> Vec<String> {
    let mut out = Vec::new();
    out.extend(fake);
    out.extend(live);
    if let Err(message) = report {
        out.push(message.clone());
    }
    out
}

fn emit(outcome: &Outcome) -> Result<(), String> {
    let mut stdout = std::io::stdout();
    for line in report_lines(&outcome.shapes) {
        write_line(&mut stdout, &line)?;
    }
    for line in &outcome.product {
        write_line(&mut stdout, line)?;
    }
    let note = match &outcome.report {
        Ok(path) => format!("report written to {}", path.display()),
        Err(message) => format!("report write failed: {message}"),
    };
    write_line(&mut stdout, &note)?;
    stdout.flush().map_err(|e| e.to_string())
}

fn write_line(stdout: &mut std::io::Stdout, line: &str) -> Result<(), String> {
    writeln!(stdout, "{line}").map_err(|e| e.to_string())
}

impl Outcome {
    fn failure(self) -> Result<(), String> {
        if let Some(first) = self.failures.first() {
            return Err(first.clone());
        }
        Ok(())
    }
}
