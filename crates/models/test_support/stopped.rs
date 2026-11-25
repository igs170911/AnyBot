use tokio::sync::watch;

pub(crate) fn stopped() -> (watch::Sender<bool>, watch::Receiver<bool>) {
    watch::channel(false)
}
