use std::path::{Path, PathBuf};

#[must_use]
pub fn repo_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    match manifest.join("../..").canonicalize() {
        Ok(p) => p,
        Err(_) => manifest,
    }
}

pub fn test_dir(test_name: &str) -> Result<PathBuf, String> {
    let base = repo_root().join("target").join("tests").join(test_name);
    let _reaped = crate::reap::reap_except(Some(&base));
    clean(&base)?;
    std::fs::create_dir_all(&base).map_err(|e| format!("create {} failed: {e}", base.display()))?;
    Ok(base)
}

fn clean(base: &Path) -> Result<(), String> {
    match std::fs::remove_dir_all(base) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!("clean {} failed: {e}", base.display())),
    }
}

pub fn write_file_with_mode(path: &Path, contents: &str, mode: u32) -> Result<(), String> {
    std::fs::write(path, contents).map_err(|e| format!("write {} failed: {e}", path.display()))?;
    set_mode(path, mode)
}

#[cfg(unix)]
fn set_mode(path: &Path, mode: u32) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))
        .map_err(|e| format!("set permissions on {} failed: {e}", path.display()))
}

#[cfg(not(unix))]
fn set_mode(_path: &Path, _mode: u32) -> Result<(), String> {
    Ok(())
}
