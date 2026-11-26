pub async fn wait_for_stop() -> Result<(), String> {
    bind_stop()
        .await
        .map_err(|err| format!("testenv stop signal failed: {err}"))
}

#[cfg(unix)]
async fn bind_stop() -> Result<(), std::io::Error> {
    let mut terminate = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
    tokio::select! {
        _ctrl = tokio::signal::ctrl_c() => Ok(()),
        Some(_sig) = terminate.recv() => Ok(()),
    }
}

#[cfg(windows)]
async fn bind_stop() -> Result<(), std::io::Error> {
    let mut ctrl_break = tokio::signal::windows::ctrl_break()?;
    let mut ctrl_close = tokio::signal::windows::ctrl_close()?;
    tokio::select! {
        _ctrl = tokio::signal::ctrl_c() => Ok(()),
        _break = ctrl_break.recv() => Ok(()),
        _close = ctrl_close.recv() => Ok(()),
    }
}
