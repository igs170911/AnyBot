use std::sync::Arc;

pub(crate) fn watch_stop_signals(shutdown: &Arc<tokio::sync::Notify>) {
    let stop = Arc::clone(shutdown);
    tokio::spawn(async move {
        let _reported = listen_for_stop()
            .await
            .map_or_else(|err| err.to_string(), |()| "stopped".to_owned());
        stop.notify_waiters();
    });
}

#[cfg(unix)]
async fn listen_for_stop() -> Result<(), std::io::Error> {
    let mut terminate = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
    tokio::select! {
        _ctrl = tokio::signal::ctrl_c() => Ok(()),
        Some(_sig) = terminate.recv() => Ok(()),
    }
}

#[cfg(windows)]
async fn listen_for_stop() -> Result<(), std::io::Error> {
    let mut ctrl_break = tokio::signal::windows::ctrl_break()?;
    let mut ctrl_close = tokio::signal::windows::ctrl_close()?;
    tokio::select! {
        _ctrl = tokio::signal::ctrl_c() => Ok(()),
        _break = ctrl_break.recv() => Ok(()),
        _close = ctrl_close.recv() => Ok(()),
    }
}
