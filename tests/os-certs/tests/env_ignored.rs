use std::io::Write;
use std::path::Path;
use std::process::Command;

use anybot_foundation as _;

#[test]
fn ssl_cert_env_does_not_change_the_os_store() -> Result<(), String> {
    let dir = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("m7a5-cert-env");
    let cert_dir = dir.join("certs");
    std::fs::create_dir_all(&cert_dir).map_err(|err| err.to_string())?;
    let file = dir.join("only-probe.pem");
    write_probe(&file)?;
    std::fs::copy(&file, cert_dir.join("probe.pem")).map_err(|err| err.to_string())?;
    let before = run_loader(&dir, None, None)?;
    let during = run_loader(&dir, Some(file.as_path()), Some(cert_dir.as_path()))?;
    assert_eq!(during, before);
    assert!(before > 1, "{before}");
    Ok(())
}

fn write_probe(file: &Path) -> Result<(), String> {
    let issued = rcgen::generate_simple_self_signed(vec!["probe.invalid".to_owned()])
        .map_err(|err| err.to_string())?;
    let mut written = std::fs::File::create(file).map_err(|err| err.to_string())?;
    written
        .write_all(issued.cert.pem().as_bytes())
        .map_err(|err| err.to_string())
}

fn run_loader(dir: &Path, file: Option<&Path>, cert_dir: Option<&Path>) -> Result<usize, String> {
    let mut command = Command::new(env!("CARGO_BIN_EXE_os-certs"));
    command.current_dir(dir);
    if let Some(file) = file {
        command.env("SSL_CERT_FILE", file);
    }
    if let Some(cert_dir) = cert_dir {
        command.env("SSL_CERT_DIR", cert_dir);
    }
    let output = command.output().map_err(|err| err.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).into_owned());
    }
    let text = String::from_utf8_lossy(&output.stdout);
    text.trim().parse::<usize>().map_err(|err| err.to_string())
}
