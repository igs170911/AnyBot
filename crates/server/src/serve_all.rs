use axum::Router;
use tokio::net::TcpListener;
use tokio::sync::mpsc;

use anybot_foundation::MessageRef;

pub const CODE_SERVER_STOP_NORMAL: &str = "server-stop-normal";
pub const CODE_SERVER_STOP_ERROR: &str = "server-stop-error";
pub const CODE_SERVER_STOP_NO_LISTENER: &str = "server-stop-no-listener";

fn stop_reason(result: Result<(), std::io::Error>) -> MessageRef {
    match result {
        Ok(()) => MessageRef::new(CODE_SERVER_STOP_NORMAL),
        Err(err) => MessageRef::new(CODE_SERVER_STOP_ERROR).with_text("reason", &err.to_string()),
    }
}

pub async fn serve_all(listeners: Vec<TcpListener>, app: Router) -> Result<MessageRef, MessageRef> {
    let (sender, mut receiver) = mpsc::channel::<MessageRef>(16);
    let mut tasks = Vec::new();
    for listener in listeners {
        let sender = sender.clone();
        let app = app.clone();
        tasks.push(tokio::spawn(async move {
            let result = axum::serve(listener, app).await;
            let _result = sender.send(stop_reason(result)).await;
        }));
    }
    drop(sender);
    let reason = receiver
        .recv()
        .await
        .ok_or(MessageRef::new(CODE_SERVER_STOP_NO_LISTENER))?;
    for task in tasks {
        task.abort();
    }
    Ok(reason)
}
