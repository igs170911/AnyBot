use std::path::{Path, PathBuf};

pub(crate) fn resolve_missing(joined: &Path) -> Result<PathBuf, std::io::Error> {
    let mut tail: Vec<std::ffi::OsString> = Vec::new();
    let mut cursor = joined.to_path_buf();
    loop {
        if let Some(name) = cursor.file_name() {
            tail.push(name.to_os_string());
        }
        let Some(parent) = cursor.parent() else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "no existing ancestor folder",
            ));
        };
        if let Ok(canonical) = std::fs::canonicalize(parent) {
            let mut out = canonical;
            for name in tail.iter().rev() {
                out.push(name);
            }
            return Ok(out);
        }
        cursor = parent.to_path_buf();
    }
}
