use std::io::Write;
use std::process::ExitCode;

#[cfg(test)]
use rcgen as _;

fn main() -> ExitCode {
    match load() {
        Ok(count) => {
            let mut out = std::io::stdout();
            match writeln!(out, "{count}") {
                Ok(()) => ExitCode::SUCCESS,
                Err(err) => fail(&err.to_string()),
            }
        }
        Err(message) => fail(&message),
    }
}

fn fail(message: &str) -> ExitCode {
    let mut err = std::io::stderr();
    match writeln!(err, "{message}") {
        Ok(()) | Err(_) => ExitCode::from(1),
    }
}

fn load() -> Result<usize, String> {
    anybot_foundation::TrustStore::new(&anybot_foundation::TrustedCerts::empty())
        .map(|store| store.root_count())
}
