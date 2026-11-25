use std::io::Write as _;
use std::sync::Arc;

use anybot_messages::{Catalog, Lang, MessageCode, Params};
use test_harness::{ArcWriter, Capture};

use crate::serve::TEST_VERSION;

const CODE_STARTUP_LISTENING: &str = "server-startup-listening";
const CODE_STARTUP_NO_ACCOUNTS: &str = "server-startup-no-accounts";
const CODE_STARTUP_INVITE_GENERATED: &str = "server-startup-invite-generated";

pub fn startup_capture(
    started: &anybot_server::Started,
    port: u16,
) -> Result<Arc<Capture>, String> {
    let sink = Arc::new(Capture::new());
    let mut writer = ArcWriter {
        sink: Arc::clone(&sink),
    };
    let listening = Params::new()
        .with_text("version", TEST_VERSION)
        .with_text("addresses", &format!("127.0.0.1:{port}, [::1]:{port}"))
        .with_text("db_host", "127.0.0.1")
        .with_text("db_name", "anybot_test");
    emit_line(CODE_STARTUP_LISTENING, &listening, &mut writer)?;
    if started.account_count == 0 {
        emit_line(CODE_STARTUP_NO_ACCOUNTS, &Params::new(), &mut writer)?;
    }
    if let Some(code) = &started.new_invite {
        let drawn = Params::new().with_text("code", code);
        emit_line(CODE_STARTUP_INVITE_GENERATED, &drawn, &mut writer)?;
    }
    Ok(sink)
}

fn emit_line(code: &str, params: &Params, writer: &mut ArcWriter) -> Result<(), String> {
    let found = MessageCode::from_code(code).ok_or(format!("unknown startup code {code}"))?;
    let mut catalog = Catalog::new();
    let text = catalog
        .format(Lang::En, &found, params)
        .map_err(|e| e.to_string())?;
    writer
        .write_all(text.as_bytes())
        .map_err(|e| e.to_string())?;
    writer.write_all(b"\n").map_err(|e| e.to_string())?;
    Ok(())
}
