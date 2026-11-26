use std::path::PathBuf;

use anybot_foundation::ContentDigest;
use anybot_protocol::{
    ActualResult, BeforeDigest, CommandOutcome, ComputerCaps, ComputerFailure, ComputerOwner,
    ComputerPlatform, PathOutcome, ReadOutcome, TargetKind, WriteOutcome,
};
use anybot_trust::{GatedComputer, OpRejection, Pass, apply_edit};

use crate::command::run_line;
use crate::fs_helpers::{
    cap_pair, edit_failure, read_bytes, size_u64, target_kind_of, text_of, write_content,
};
use crate::paths::resolve_missing;

pub struct BotComputer {
    name: String,
    platform: ComputerPlatform,
    owner: ComputerOwner,
    root: PathBuf,
}

impl BotComputer {
    pub fn new(
        name: &str,
        platform: ComputerPlatform,
        owner: ComputerOwner,
        root: &str,
    ) -> Result<Self, ComputerFailure> {
        let root = PathBuf::from(root);
        std::fs::create_dir_all(&root)
            .map_err(|e| ComputerFailure::operation(format!("cannot prepare the root: {e}")))?;
        Ok(BotComputer {
            name: name.to_owned(),
            platform,
            owner,
            root,
        })
    }

    fn full_path(&self, path: &str) -> Result<PathBuf, OpRejection> {
        let root = std::fs::canonicalize(&self.root).map_err(|e| {
            OpRejection::from_failure(ComputerFailure::operation(format!("cannot resolve: {e}")))
        })?;
        let joined = root.join(path);
        let resolved = if joined.exists() {
            std::fs::canonicalize(&joined)
        } else {
            resolve_missing(&joined)
        }
        .map_err(|e| {
            OpRejection::from_failure(ComputerFailure::operation(format!("cannot resolve: {e}")))
        })?;
        if !resolved.starts_with(&root) {
            return Err(OpRejection::from_failure(ComputerFailure::operation(
                "the resolved path leaves the workspace".to_owned(),
            )));
        }
        Ok(resolved)
    }
}

impl GatedComputer for BotComputer {
    fn name(&self) -> &str {
        &self.name
    }

    fn declared_caps(&self) -> ComputerCaps {
        let mut caps = ComputerCaps::declared(self.platform, self.owner);
        caps.command_sandbox = cap_pair(true);
        caps.run_commands = cap_pair(true);
        caps.read_files = cap_pair(true);
        caps.write_files = cap_pair(true);
        caps.check_path = cap_pair(true);
        caps
    }

    fn capabilities(&self, _pass: &Pass) -> Result<ActualResult, OpRejection> {
        Ok(ActualResult::Caps(self.declared_caps()))
    }

    fn resolve_target(&self, path: &str) -> Option<String> {
        let full = self.full_path(path).ok()?;
        Some(full.display().to_string())
    }

    fn path_info(&self, _pass: &Pass, path: &str) -> Result<ActualResult, OpRejection> {
        let full = self.full_path(path)?;
        Ok(ActualResult::Path(PathOutcome {
            path: path.to_owned(),
            canonical: full.display().to_string(),
            kind: target_kind_of(&full),
        }))
    }

    fn read_file(&self, _pass: &Pass, path: &str) -> Result<ActualResult, OpRejection> {
        let full = self.full_path(path)?;
        let content = text_of(read_bytes(&full)?)?;
        Ok(ActualResult::Read(ReadOutcome {
            path: full.display().to_string(),
            kind: TargetKind::File,
            digest: ContentDigest::sha256(content.as_bytes()),
            size: size_u64(content.len())?,
            content,
        }))
    }

    fn write_file(
        &self,
        _pass: &Pass,
        path: &str,
        content: &str,
    ) -> Result<ActualResult, OpRejection> {
        let full = self.full_path(path)?;
        let before = match read_bytes(&full) {
            Ok(bytes) => BeforeDigest::Present(ContentDigest::sha256(&bytes)),
            Err(_) => BeforeDigest::Absent,
        };
        write_content(&full, content)?;
        Ok(ActualResult::Write(WriteOutcome {
            path: full.display().to_string(),
            size: size_u64(content.len())?,
            before,
            after: ContentDigest::sha256(content.as_bytes()),
        }))
    }

    fn edit_file(
        &self,
        pass: &Pass,
        path: &str,
        old_text: &str,
        new_text: &str,
    ) -> Result<ActualResult, OpRejection> {
        let current = self.current_content(pass, path, old_text)?;
        let applied =
            apply_edit(current.as_deref(), old_text, new_text).map_err(|e| edit_failure(&e))?;
        self.write_file(pass, path, &applied.content)
    }

    fn run_command(
        &self,
        _pass: &Pass,
        line: &str,
        timeout_ms: u64,
    ) -> Result<ActualResult, OpRejection> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| {
                OpRejection::from_failure(ComputerFailure::operation(format!(
                    "cannot start the runtime: {e}"
                )))
            })?;
        let outcome = runtime.block_on(run_line(line, &self.root, timeout_ms))?;
        Ok(ActualResult::Command(CommandOutcome {
            command: line.to_owned(),
            computer: self.name.clone(),
            exit_code: outcome.0,
            output: outcome.1,
            elapsed_ms: outcome.2,
        }))
    }
}

impl BotComputer {
    fn current_content(
        &self,
        pass: &Pass,
        path: &str,
        old_text: &str,
    ) -> Result<Option<String>, OpRejection> {
        match self.read_file(pass, path) {
            Ok(ActualResult::Read(read)) => Ok(Some(read.content)),
            Ok(_) => Err(OpRejection::from_failure(ComputerFailure::operation(
                "the path is not a file".to_owned(),
            ))),
            Err(rejection) => {
                if old_text.is_empty() {
                    Ok(None)
                } else {
                    Err(rejection)
                }
            }
        }
    }
}
