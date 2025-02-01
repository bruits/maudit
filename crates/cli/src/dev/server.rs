use axum::{
    body::to_bytes,
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Request, State,
    },
    handler::HandlerWithoutStateExt,
    http::{HeaderValue, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use tokio::{signal, sync::broadcast};

use std::net::SocketAddr;
use tower_http::{
    services::ServeDir,
    trace::{DefaultMakeSpan, TraceLayer},
};

use axum::extract::connect_info::ConnectInfo;
use futures::{stream::StreamExt, SinkExt};

#[derive(Clone, Debug)]
pub struct WebSocketMessage {
    pub data: String,
}

#[derive(Clone)]
struct AppState {
    tx: broadcast::Sender<WebSocketMessage>,
}

pub async fn start_dev_web_server(tx: broadcast::Sender<WebSocketMessage>) {
    async fn handle_404() -> (StatusCode, &'static str) {
        (StatusCode::NOT_FOUND, "Not found")
    }

    let service = handle_404.into_service();
    let serve_dir = ServeDir::new("dist").not_found_service(service);

    let router = Router::new()
        .route("/ws", get(ws_handler))
        .fallback_service(serve_dir)
        .layer(middleware::from_fn(add_dev_client_script))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::default().include_headers(true)),
        )
        .with_state(AppState { tx });

    // run it with hyper
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();

    tracing::debug!("listening on {}", listener.local_addr().unwrap());
    axum::serve(
        listener,
        router.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await
    .unwrap();
}

async fn add_dev_client_script(req: Request, next: Next) -> Response {
    let res = next.run(req).await;

    if res.headers().get(axum::http::header::CONTENT_TYPE)
        == Some(&HeaderValue::from_static("text/html"))
    {
        let body = res.into_body();
        let bytes = to_bytes(body, usize::MAX).await.unwrap();

        let mut body = String::from_utf8_lossy(&bytes).into_owned();

        body.push_str(&format!("<script>{}</script>", include_str!("./client.js")));

        // Copy the headers from the original response
        let mut res = Response::new(body.into());
        *res.headers_mut() = res.headers().clone();

        return res;
    }

    res
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    println!("`{addr} connected.");
    // finalize the upgrade process by returning upgrade callback.
    // we can customize the callback by sending additional info such as address.
    ws.on_upgrade(move |socket| handle_socket(socket, addr, state.tx))
}

async fn handle_socket(
    socket: WebSocket,
    who: SocketAddr,
    tx: broadcast::Sender<WebSocketMessage>,
) {
    let (mut sender, mut receiver) = socket.split();

    let mut rx = tx.subscribe();

    tokio::select! {
        _ = async {
            while let Some(Ok(msg)) = receiver.next().await {
                match msg {
                    Message::Text(_) => {}
                    Message::Binary(_) => {
                    }
                    _ => {}
                }
            }
        } => {},
        _ = async {
            while let Ok(msg) = rx.recv().await {
                println!(">>> got messages from higher level: {0}", msg.data);
                let _ = sender.send(Message::Text(msg.data.into())).await;
            }
        } => {},
    }

    // returning from the handler closes the websocket connection
    println!("Websocket context {who} destroyed");
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
