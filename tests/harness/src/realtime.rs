use std::future::Future;
use std::time::Duration;

pub async fn sleep_real(span: Duration) {
    tokio::time::sleep(span).await;
}

pub async fn timeout_real<F>(fut: F, span: Duration) -> Result<F::Output, String>
where
    F: Future,
{
    let result = tokio::time::timeout(span, fut).await;
    result.map_err(|_e| "real-time deadline expired".to_owned())
}
