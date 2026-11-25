use std::io::Write;

use anybot_messages::{Catalog, Lang, MessageCode, Params};

#[derive(Debug)]
pub struct StartupLine {
    pub code: MessageCode,
    pub params: Params,
}

impl StartupLine {
    pub const CODE: MessageCode = MessageCode::ServerStartupListening;

    #[must_use]
    pub fn new(params: Params) -> Self {
        StartupLine {
            code: Self::CODE,
            params,
        }
    }
}

#[derive(Debug)]
pub struct StopLine {
    pub code: MessageCode,
    pub params: Params,
}

impl StopLine {
    pub const CODE_NORMAL: MessageCode = MessageCode::ServerStopNormal;
    pub const CODE_ERROR: MessageCode = MessageCode::ServerStopError;

    #[must_use]
    pub fn new(code: MessageCode, params: Params) -> Self {
        StopLine { code, params }
    }
}

#[derive(Debug)]
pub struct FailureLine {
    pub code: MessageCode,
    pub params: Params,
}

impl FailureLine {
    #[must_use]
    pub fn new(code: MessageCode, params: Params) -> Self {
        FailureLine { code, params }
    }
}

pub trait Line {
    fn code(&self) -> MessageCode;

    fn params(&self) -> &Params;
}

impl Line for StartupLine {
    fn code(&self) -> MessageCode {
        self.code
    }

    fn params(&self) -> &Params {
        &self.params
    }
}

impl Line for StopLine {
    fn code(&self) -> MessageCode {
        self.code
    }

    fn params(&self) -> &Params {
        &self.params
    }
}

impl Line for FailureLine {
    fn code(&self) -> MessageCode {
        self.code
    }

    fn params(&self) -> &Params {
        &self.params
    }
}

#[must_use]
pub fn line_text<L: Line>(catalog: &mut Catalog, line: &L) -> String {
    let code = line.code();
    catalog
        .format(Lang::default_lang(), &code, line.params())
        .unwrap_or_else(|err| format!("{code:?} formatter failure: {err}"))
}

pub fn emit(text: &str, out: &mut impl Write) -> std::io::Result<()> {
    writeln!(out, "{text}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn startup_params() -> Params {
        Params::new()
            .with_text("version", "9.9.9")
            .with_int("port", 8787)
            .with_text("db_host", "127.0.0.1")
            .with_text("db_name", "anybot_next_test")
    }

    #[test]
    fn startup_line_text_uses_default_english() {
        let mut catalog = Catalog::new();
        let text = line_text(&mut catalog, &StartupLine::new(startup_params()));
        assert!(text.contains("AnyBot server v9.9.9"), "{text}");
        assert!(text.contains("127.0.0.1:8787"), "{text}");
    }

    #[test]
    fn stop_line_codes_are_distinct() {
        assert_ne!(StopLine::CODE_NORMAL, StopLine::CODE_ERROR);
        let line = StopLine::new(StopLine::CODE_NORMAL, Params::new());
        let mut catalog = Catalog::new();
        let text = line_text(&mut catalog, &line);
        assert!(text.contains("ended normally"), "{text}");
    }

    #[test]
    fn failure_line_maps_mode_and_path_params() {
        let mut catalog = Catalog::new();
        let line = FailureLine::new(
            MessageCode::ServerBootstrapPermissionsTooOpen,
            Params::new()
                .with_text("mode", "644")
                .with_text("path", "start.toml"),
        );
        let text = line_text(&mut catalog, &line);
        assert!(text.contains("644"), "{text}");
        assert!(text.contains("start.toml"), "{text}");
    }

    #[test]
    fn emit_appends_one_line_per_call() {
        let mut out = Vec::new();
        emit("first", &mut out).unwrap();
        emit("second", &mut out).unwrap();
        assert_eq!(String::from_utf8(out).unwrap(), "first\nsecond\n");
    }
}
