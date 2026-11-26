use std::path::{Path, PathBuf};
use std::process::Command;

const LABEL: &str = "target/permit-mint-blocked";
const CALL: &str = "pub fn mint_outside_models() -> Option<anybot_models::SendPermit> {\n    let endpoint = anybot_models::resolve_endpoint(anybot_models::ProviderKind::Custom, \"http://127.0.0.1:9/v1\", \"\", \"m\", None);\n    anybot_models::SendPermit::for_endpoint(&anybot_models::DataPolicy::LocalOnly, &endpoint, anybot_models::ModelUse::Bot)\n}\n";
const PIN_LABEL: &str = "target/pin-dial-blocked";
const PIN_CALL: &str = "pub fn pin_outside_models() -> Result<(), String> {\n    let clock: std::sync::Arc<dyn anybot_foundation::clock::Clock> = std::sync::Arc::new(anybot_foundation::clock::SystemClock);\n    let mut gateway = anybot_models::gateway(clock, &anybot_foundation::TrustedCerts::empty())?;\n    gateway.pin_host(\"api.cloud.test\", std::net::SocketAddr::from(([127, 0, 0, 1], 9)));\n    Ok(())\n}\n";

#[must_use]
pub fn violations(repo: &Path) -> Vec<String> {
    let pkg = match prepare(repo) {
        Ok(pkg) => pkg,
        Err(err) => return vec![err],
    };
    let judged = judge(clippy_output(repo, &pkg));
    let _removed = std::fs::remove_dir_all(repo.join("target/permit-mint-blocked"));
    let mut all = judged;
    all.extend(pin_violations(repo));
    all
}

fn pin_violations(repo: &Path) -> Vec<String> {
    let pkg = match prepare_pin(repo) {
        Ok(pkg) => pkg,
        Err(err) => return vec![err],
    };
    let judged = judge_pin(clippy_output(repo, &pkg));
    let _removed = std::fs::remove_dir_all(repo.join(PIN_LABEL));
    judged
}

fn prepare(repo: &Path) -> Result<PathBuf, String> {
    let scratch = repo.join("target/permit-mint-blocked");
    let pkg = scratch.join("permit-mint-blocked");
    let src = pkg.join("src");
    std::fs::create_dir_all(&src).map_err(|err| format!("{LABEL}: prepare: {err}"))?;
    write_example(repo, &scratch, &pkg, &src)?;
    Ok(pkg)
}

fn prepare_pin(repo: &Path) -> Result<PathBuf, String> {
    let scratch = repo.join(PIN_LABEL);
    let pkg = scratch.join("pin-dial-blocked");
    let src = pkg.join("src");
    std::fs::create_dir_all(&src).map_err(|err| format!("{PIN_LABEL}: prepare: {err}"))?;
    write_pin(repo, &scratch, &pkg, &src)?;
    Ok(pkg)
}

fn write_pin(repo: &Path, scratch: &Path, pkg: &Path, src: &Path) -> Result<(), String> {
    let workspace = "[workspace]\nmembers = [\"pin-dial-blocked\"]\nresolver = \"2\"\n";
    let manifest = "[package]\nname = \"pin-dial-blocked\"\nversion = \"0.0.0\"\nedition = \"2024\"\npublish = false\n[dependencies]\nanybot-models = { path = \"../../../crates/models\" }\nanybot-foundation = { path = \"../../../crates/foundation\" }\n";
    std::fs::write(scratch.join("Cargo.toml"), workspace)
        .and_then(|()| std::fs::write(pkg.join("Cargo.toml"), manifest))
        .and_then(|()| std::fs::write(src.join("lib.rs"), PIN_CALL))
        .map_err(|err| format!("{PIN_LABEL}: write: {err}"))?;
    std::fs::copy(
        repo.join("crates/sync/clippy.toml"),
        pkg.join("clippy.toml"),
    )
    .map(|_| ())
    .map_err(|err| format!("{PIN_LABEL}: clippy.toml: {err}"))
}

fn write_example(repo: &Path, scratch: &Path, pkg: &Path, src: &Path) -> Result<(), String> {
    let workspace = "[workspace]\nmembers = [\"permit-mint-blocked\"]\nresolver = \"2\"\n";
    let manifest = "[package]\nname = \"permit-mint-blocked\"\nversion = \"0.0.0\"\nedition = \"2024\"\npublish = false\n[dependencies]\nanybot-models = { path = \"../../../crates/models\" }\n";
    std::fs::write(scratch.join("Cargo.toml"), workspace)
        .and_then(|()| std::fs::write(pkg.join("Cargo.toml"), manifest))
        .and_then(|()| std::fs::write(src.join("lib.rs"), CALL))
        .map_err(|err| format!("{LABEL}: write: {err}"))?;
    std::fs::copy(
        repo.join("crates/sync/clippy.toml"),
        pkg.join("clippy.toml"),
    )
    .map(|_| ())
    .map_err(|err| format!("{LABEL}: clippy.toml: {err}"))
}

fn clippy_output(repo: &Path, pkg: &Path) -> Result<std::process::Output, String> {
    Command::new("cargo")
        .arg("clippy")
        .arg("--offline")
        .arg("--manifest-path")
        .arg(pkg.join("Cargo.toml"))
        .arg("--target-dir")
        .arg(repo.join("target/permit-mint-target"))
        .arg("--")
        .arg("-D")
        .arg("warnings")
        .arg("-W")
        .arg("clippy::disallowed_methods")
        .current_dir(repo)
        .output()
        .map_err(|err| format!("{LABEL}: cargo clippy failed to start: {err}"))
}

fn judge_pin(output: Result<std::process::Output, String>) -> Vec<String> {
    let output = match output {
        Ok(output) => output,
        Err(err) => return vec![err],
    };
    let combined = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    if output.status.success() {
        return vec![format!(
            "{PIN_LABEL}: clippy allowed Gateway::pin_host outside models"
        )];
    }
    if combined.contains("disallowed_methods") && combined.contains("Gateway::pin_host") {
        return Vec::new();
    }
    vec![format!(
        "{PIN_LABEL}: clippy failed for a reason other than the pin ban"
    )]
}

fn judge(output: Result<std::process::Output, String>) -> Vec<String> {
    let output = match output {
        Ok(output) => output,
        Err(err) => return vec![err],
    };
    let combined = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    if output.status.success() {
        return vec![format!(
            "{LABEL}: clippy allowed SendPermit::for_endpoint outside models"
        )];
    }
    if combined.contains("disallowed_methods") && combined.contains("SendPermit::for_endpoint") {
        return Vec::new();
    }
    vec![format!(
        "{LABEL}: clippy failed for a reason other than the permit ban"
    )]
}

#[cfg(test)]
mod tests {
    use super::violations;

    #[test]
    fn test_iron_laws_permit_call_is_the_banned_method() {
        let call = super::CALL;
        assert!(call.contains("SendPermit::for_endpoint"));
        assert!(super::PIN_CALL.contains("pin_host"));
        assert!(!call.contains("cfg(test)"));
    }

    #[test]
    fn test_iron_laws_permit_mint_is_rejected_by_clippy() {
        let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let repo = manifest.join("../..");
        let found = violations(&repo);
        assert!(found.is_empty(), "{found:?}");
    }
}
