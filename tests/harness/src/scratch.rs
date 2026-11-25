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
    clean(&base)?;
    std::fs::create_dir_all(&base).map_err(|e| format!("建 {} 失敗：{e}", base.display()))?;
    Ok(base)
}

fn clean(base: &Path) -> Result<(), String> {
    match std::fs::remove_dir_all(base) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!("清 {} 失敗：{e}", base.display())),
    }
}

pub fn write_file_with_mode(path: &Path, contents: &str, mode: u32) -> Result<(), String> {
    std::fs::write(path, contents).map_err(|e| format!("寫 {} 失敗：{e}", path.display()))?;
    set_mode(path, mode)
}

#[cfg(unix)]
fn set_mode(path: &Path, mode: u32) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))
        .map_err(|e| format!("設 {} 的權限失敗：{e}", path.display()))
}

#[cfg(not(unix))]
fn set_mode(_path: &Path, _mode: u32) -> Result<(), String> {
    Ok(())
}
