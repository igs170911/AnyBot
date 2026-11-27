use crate::bootstrap::{ServerBootstrap, parse_connection};
use crate::error::{
    CODE_BOOTSTRAP_LINE_MALFORMED, CODE_BOOTSTRAP_NAME_DUPLICATE, CODE_BOOTSTRAP_NAME_MISSING,
    CODE_BOOTSTRAP_NAME_UNKNOWN, FoundationError, MessageRef,
};
use crate::master_key::{MASTER_KEY_NAME, MasterKey};
use crate::secret::Secret;

pub const DATABASE_NAME: &str = "database";

const NAME_MAX_LEN: usize = 32;

#[derive(Default)]
struct Entries {
    database: Option<Secret>,
    master_key: Option<Secret>,
}

pub fn parse_bootstrap(text: &str) -> Result<ServerBootstrap, FoundationError> {
    let mut entries = Entries::default();
    for (index, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        let (name, value) = split_entry(line, index.saturating_add(1))?;
        let slot = match name {
            DATABASE_NAME => &mut entries.database,
            MASTER_KEY_NAME => &mut entries.master_key,
            other => return Err(named(CODE_BOOTSTRAP_NAME_UNKNOWN, other)),
        };
        if slot.is_some() {
            return Err(named(CODE_BOOTSTRAP_NAME_DUPLICATE, name));
        }
        *slot = Some(Secret::from_untrusted(value.to_owned()));
    }
    let database = entries
        .database
        .ok_or_else(|| named(CODE_BOOTSTRAP_NAME_MISSING, DATABASE_NAME))?;
    let master = entries
        .master_key
        .ok_or_else(|| named(CODE_BOOTSTRAP_NAME_MISSING, MASTER_KEY_NAME))?;
    let master_key = MasterKey::from_hex(master.expose())?;
    parse_connection(database.expose(), master_key)
}

fn split_entry(line: &str, number: usize) -> Result<(&str, &str), FoundationError> {
    let Some((name, value)) = line.split_once('=') else {
        return Err(line_malformed(number));
    };
    let name = name.trim();
    if !is_plain_name(name) {
        return Err(line_malformed(number));
    }
    Ok((name, value.trim()))
}

fn is_plain_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= NAME_MAX_LEN
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

fn line_malformed(number: usize) -> FoundationError {
    FoundationError::BootstrapMalformed(
        MessageRef::new(CODE_BOOTSTRAP_LINE_MALFORMED).with_text("line", &number.to_string()),
    )
}

fn named(code: &str, name: &str) -> FoundationError {
    FoundationError::BootstrapMalformed(MessageRef::new(code).with_text("name", name))
}

#[cfg(test)]
#[path = "bootstrap_file_tests.rs"]
mod tests;
