use std::collections::BTreeMap;

use anybot_protocol::{DirEntry, TargetKind};

use crate::fake_files::{FileTree, normalize_path};

impl FileTree {
    pub fn list_entries(&self, path: &str) -> Result<Vec<DirEntry>, String> {
        let canonical = self.canonical(path);
        if self.files.contains_key(&canonical) {
            return Err(format!("{canonical} is a file"));
        }
        if !self.dirs.contains(&canonical) {
            return Err(format!("{canonical} is not a directory"));
        }
        let prefix = dir_prefix(&canonical);
        let mut out: BTreeMap<String, DirEntry> = BTreeMap::new();
        for dir_path in &self.dirs {
            if let Some(name) = immediate_child(dir_path, &prefix) {
                out.insert(name.to_owned(), entry(name, TargetKind::Directory, 0));
            }
        }
        for (file_path, content) in &self.files {
            if let Some(name) = immediate_child(file_path, &prefix) {
                let size = content.len() as u64;
                out.insert(name.to_owned(), entry(name, TargetKind::File, size));
            }
        }
        Ok(out.into_values().collect())
    }

    pub fn search_files(&self, path: &str) -> Result<Vec<(String, String)>, String> {
        let canonical = self.canonical(path);
        if let Some(content) = self.files.get(&canonical) {
            let display = display_rel_path(&self.workdir, "/", &canonical);
            return Ok(vec![(display, content.clone())]);
        }
        if !self.dirs.contains(&canonical) {
            return Err(format!("{canonical} is not a directory"));
        }
        let dir_prefix = dir_prefix(&canonical);
        let workdir_prefix = dir_prefix_for(&self.workdir);
        let mut out = Vec::new();
        for (file_path, content) in &self.files {
            if file_path.starts_with(&dir_prefix) {
                let display = display_rel_path(&workdir_prefix, &dir_prefix, file_path);
                out.push((display, content.clone()));
            }
        }
        Ok(out)
    }
}

fn entry(name: &str, kind: TargetKind, size: u64) -> DirEntry {
    DirEntry {
        name: name.to_owned(),
        kind,
        size,
    }
}

fn immediate_child<'a>(path: &'a str, prefix: &str) -> Option<&'a str> {
    let rest = path.strip_prefix(prefix)?;
    if rest.is_empty() || rest.contains('/') {
        return None;
    }
    Some(rest)
}

fn dir_prefix(canonical: &str) -> String {
    if canonical == "/" {
        "/".to_owned()
    } else {
        format!("{canonical}/")
    }
}

fn dir_prefix_for(workdir: &str) -> String {
    dir_prefix(&normalize_path(workdir))
}

fn display_rel_path(workdir_prefix: &str, dir_prefix: &str, file_path: &str) -> String {
    if let Some(rel) = file_path.strip_prefix(workdir_prefix) {
        return rel.to_owned();
    }
    if let Some(rel) = file_path.strip_prefix(dir_prefix) {
        return rel.to_owned();
    }
    file_path
        .rsplit_once('/')
        .map_or_else(|| file_path.to_owned(), |(_, name)| name.to_owned())
}
