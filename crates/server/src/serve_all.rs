use axum::Router;
use tokio::net::TcpListener;
use tokio::sync::mpsc;

fn stop_reason(result: Result<(), std::io::Error>) -> String {
    match result {
        Ok(()) => "其中一個位址的伺服器正常結束".to_owned(),
        Err(err) => format!("伺服器停止：{err}"),
    }
}

pub async fn serve_all(listeners: Vec<TcpListener>, app: Router) -> String {
    let (sender, mut receiver) = mpsc::channel::<String>(16);
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
        .unwrap_or("沒有任何位址的伺服器在服務".to_owned());
    for task in tasks {
        task.abort();
    }
    reason
}
