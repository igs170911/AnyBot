use std::io::{Read, Write};
use std::path::Path;
use std::process::ExitCode;

use anybot_versions::VersionSources;
use serde_json as _;

fn main() -> ExitCode {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let cargo = read_rel(&repo, "Cargo.toml");
    let tauri = read_rel(&repo, "apps/desktop/src-tauri/tauri.conf.json");
    let desktop = read_rel(&repo, "apps/desktop/package.json");
    let root_package = read_rel(&repo, "package.json");
    let release = read_rel(&repo, "crates/foundation/src/release.rs");
    let violations = anybot_versions::violations(&VersionSources {
        cargo: &cargo,
        tauri: &tauri,
        desktop: &desktop,
        root_package: &root_package,
        release: &release,
    });
    let mut stdout = std::io::stdout();
    if violations.is_empty() {
        if writeln!(stdout, "versions OK").is_err() {
            return ExitCode::FAILURE;
        }
        return ExitCode::SUCCESS;
    }
    for item in &violations {
        if writeln!(stdout, "{item}").is_err() {
            return ExitCode::FAILURE;
        }
    }
    ExitCode::from(1)
}

fn read_rel(repo: &Path, rel: &str) -> String {
    let Ok(mut file) = std::fs::File::open(repo.join(rel)) else {
        return String::new();
    };
    let mut buf = Vec::new();
    if file.read_to_end(&mut buf).is_err() {
        return String::new();
    }
    String::from_utf8_lossy(&buf).into_owned()
}
