use std::sync::Arc;

use test_harness::{ArcWriter, Capture};

pub fn capture_server_logs() -> Result<Arc<Capture>, String> {
    let sink = Arc::new(Capture::new());
    let writer_sink = Arc::clone(&sink);
    let subscriber = tracing_subscriber::fmt()
        .with_ansi(false)
        .with_max_level(tracing::Level::INFO)
        .with_writer(move || ArcWriter {
            sink: Arc::clone(&writer_sink),
        })
        .finish();
    tracing::subscriber::set_global_default(subscriber).map_err(|e| e.to_string())?;
    Ok(sink)
}
