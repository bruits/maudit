use axum::{
    body::{to_bytes, Body},
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
use quanta::Instant;
use tokio::{net::TcpSocket, signal, sync::broadcast};
use tracing::{debug, Level};

use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use tower_http::{
    services::ServeDir,
    trace::{DefaultMakeSpan, TraceLayer},
};

use axum::extract::connect_info::ConnectInfo;
use futures::{stream::StreamExt, SinkExt};

use crate::server_utils::{find_open_port, log_server_start, CustomOnResponse};
use axum::http::header;
use local_ip_address::local_ip;
use tokio::fs;

#[derive(Clone, Debug)]
pub struct WebSocketMessage {
    pub data: String,
}

#[derive(Clone)]
struct AppState {
    tx: broadcast::Sender<WebSocketMessage>,
    current_status: Arc<tokio::sync::RwLock<Option<String>>>,
}

fn inject_live_reload_script(html_content: &str, socket_addr: SocketAddr, host: bool) -> String {
    let mut content = html_content.to_string();

    let script_content = include_str!("./client.js").replace(
        "{SERVER_ADDRESS}",
        &format!(
            "{}:{}",
            if !host {
                socket_addr.ip().to_string()
            } else {
                local_ip().unwrap().to_string()
            },
            &socket_addr.port().to_string()
        ),
    );

    content.push_str(&format!("<script>{script_content}</script>"));
    content
}

pub async fn start_dev_web_server(
    start_time: Instant,
    tx: broadcast::Sender<WebSocketMessage>,
    host: bool,
    initial_error: Option<String>,
    current_status: Arc<tokio::sync::RwLock<Option<String>>>,
) {
    // TODO: The dist dir should be configurable
    let dist_dir = "dist";

    // Send initial error if present
    if let Some(error) = initial_error {
        let _ = tx.send(WebSocketMessage {
            data: format!(
                r#"{{"type": "error", "message": "{}"}}"#,
                error.replace("\"", "\\\"")
            ),
        });
    }

    async fn handle_404(socket_addr: SocketAddr, host: bool, dist_dir: &str) -> impl IntoResponse {
        let content = match fs::read_to_string(format!("{}/404.html", dist_dir)).await {
            Ok(custom_content) => custom_content,
            Err(_) => include_str!("./404.html").to_string(),
        };

        (
            StatusCode::NOT_FOUND,
            [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
            inject_live_reload_script(&content, socket_addr, host),
        )
            .into_response()
    }

    // run it with hyper, if --host 0.0.0.0 otherwise localhost
    let addr = if host {
        IpAddr::from([0, 0, 0, 0])
    } else {
        IpAddr::from([127, 0, 0, 1])
    };
    let port = find_open_port(&addr, 1864).await;
    let socket = TcpSocket::new_v4().unwrap();

    let socket_addr = SocketAddr::new(addr, port);
    socket.bind(socket_addr).unwrap();

    let listener = socket.listen(1024).unwrap();

    debug!("listening on {}", listener.local_addr().unwrap());

    let service = (move || handle_404(socket_addr, host, dist_dir)).into_service();
    let serve_dir = ServeDir::new(dist_dir).not_found_service(service);

    // TODO: Return a `.well-known/appspecific/com.chrome.devtools.json` for Chrome

    let router = Router::new()
        .route("/ws", get(ws_handler))
        .fallback_service(serve_dir)
        .layer(middleware::from_fn(move |req, next| {
            add_dev_client_script(req, next, socket_addr, host)
        }))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::default().include_headers(true)),
        )
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                .on_response(CustomOnResponse),
        )
        .with_state(AppState {
            tx: tx.clone(),
            current_status: current_status.clone(),
        });

    log_server_start(
        start_time,
        host,
        listener.local_addr().unwrap(),
        "Development",
    );

    axum::serve(
        listener,
        router.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await
    .unwrap();
}

pub async fn update_status(
    tx: &broadcast::Sender<WebSocketMessage>,
    current_status: Arc<tokio::sync::RwLock<Option<String>>>,
    status_type: &str,
    message: &str,
) {
    let status_message = if status_type == "success" {
        None // Clear the status on success
    } else {
        Some(message.to_string())
    };

    // Update the stored status
    {
        let mut status = current_status.write().await;
        *status = status_message;
    }

    // Send the message
    let _ = tx.send(WebSocketMessage {
        data: format!(
            r#"{{"type": "{}", "message": "{}"}}"#,
            status_type,
            message.replace("\"", "\\\"")
        ),
    });
}

async fn add_dev_client_script(
    req: Request,
    next: Next,
    socket_addr: SocketAddr,
    host: bool,
) -> Response {
    let uri = req.uri().clone();
    let mut res: axum::http::Response<Body> = next.run(req).await;

    res.extensions_mut().insert(uri.clone());

    if res.headers().get(axum::http::header::CONTENT_TYPE)
        == Some(&HeaderValue::from_static("text/html"))
    {
        let body = res.into_body();
        let bytes = to_bytes(body, usize::MAX).await.unwrap();

        let body = String::from_utf8_lossy(&bytes).into_owned();

        // TODO: Handle HTML documents with no tags, e.g. `"Hello, world"`. Appending a raw script tag will cause it to show up as text.
        let body_with_script = inject_live_reload_script(&body, socket_addr, host);

        // Copy the headers from the original response
        let mut res = Response::new(body_with_script.into());
        *res.headers_mut() = res.headers().clone();

        res.extensions_mut().insert(uri);

        return res;
    }

    res
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    debug!("`{addr} connected.");
    // finalize the upgrade process by returning upgrade callback.
    // we can customize the callback by sending additional info such as address.
    ws.on_upgrade(move |socket| handle_socket(socket, addr, state.tx, state.current_status))
}

async fn handle_socket(
    socket: WebSocket,
    who: SocketAddr,
    tx: broadcast::Sender<WebSocketMessage>,
    current_status: Arc<tokio::sync::RwLock<Option<String>>>,
) {
    let (mut sender, mut receiver) = socket.split();

    // Send current status to new connection if there is one
    {
        let status = current_status.read().await;
        if let Some(error_message) = status.as_ref() {
            let _ = sender
                .send(Message::Text(
                    format!(
                        r#"{{"type": "error", "message": "{}"}}"#,
                        error_message.replace("\"", "\\\"")
                    )
                    .into(),
                ))
                .await;
        }
    }

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
                debug!(">>> got messages from higher level: {0}", msg.data);
                let _ = sender.send(Message::Text(msg.data.into())).await;
            }
        } => {},
    }

    // returning from the handler closes the websocket connection
    debug!("Websocket context {who} destroyed");
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
