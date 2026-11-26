use std::collections::BTreeMap;

use anybot_trust::normalize_command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandScript {
    pub output: String,
    pub exit_code: i32,
    pub duration_ms: u64,
    pub writes: Vec<(String, String)>,
}

impl CommandScript {
    #[must_use]
    pub fn ok(output: &str) -> Self {
        CommandScript {
            output: output.to_owned(),
            exit_code: 0,
            duration_ms: 10,
            writes: Vec::new(),
        }
    }

    #[must_use]
    pub fn failing(output: &str, exit_code: i32) -> Self {
        CommandScript {
            output: output.to_owned(),
            exit_code,
            duration_ms: 10,
            writes: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_duration(self, duration_ms: u64) -> Self {
        CommandScript {
            duration_ms,
            ..self
        }
    }

    #[must_use]
    pub fn writing_to(self, path: &str, content: &str) -> Self {
        let mut writes = self.writes;
        writes.push((path.to_owned(), content.to_owned()));
        CommandScript { writes, ..self }
    }
}

#[derive(Debug, Default, Clone)]
pub struct ScriptTable {
    rows: BTreeMap<String, CommandScript>,
}

impl ScriptTable {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, command: &str, script: CommandScript) {
        self.rows.insert(normalize_command(command), script);
    }

    #[must_use]
    pub fn lookup(&self, command: &str) -> Option<&CommandScript> {
        self.rows.get(&normalize_command(command))
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_script_table_matches_commands_by_normalized_text() {
        let mut table = ScriptTable::new();
        table.insert("LS   -la", CommandScript::ok("files"));
        assert_eq!(
            table.lookup("ls -la").map(|s| s.output.as_str()),
            Some("files")
        );
        assert_eq!(
            table.lookup("ls  -LA"),
            Some(table.lookup("ls -la").unwrap())
        );
        assert!(table.lookup("pwd").is_none());
    }
}
