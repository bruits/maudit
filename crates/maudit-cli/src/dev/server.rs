use axum::{
    Router,
    body::{Body, to_bytes},
    extract::{
        Request, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::{HeaderValue, StatusCode, Uri, header::CONTENT_LENGTH},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::get,
};
use quanta::Instant;
use serde_json::json;
use tokio::{
    net::TcpSocket,
    signal,
    sync::{RwLock, broadcast},
};
use tracing::{Level, debug};

use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use tower_http::{
    services::ServeDir,
    trace::{DefaultMakeSpan, TraceLayer},
};

use axum::extract::connect_info::ConnectInfo;
use futures::{SinkExt, stream::StreamExt};

use crate::consts::PORT;
use crate::server_utils::{CustomOnResponse, find_open_port, log_server_start};
use axum::http::header;
use local_ip_address::local_ip;
use tokio::fs;

#[derive(Clone, Debug)]
pub struct WebSocketMessage {
    pub data: String,
}

#[derive(Clone, Debug)]
pub enum StatusType {
    Success,
    Info,
    Error,
}

impl std::fmt::Display for StatusType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StatusType::Success => write!(f, "success"),
            StatusType::Info => write!(f, "info"),
            StatusType::Error => write!(f, "error"),
        }
    }
}

// Persistent state for new connections
#[derive(Clone, Debug)]
pub struct PersistentStatus {
    pub status_type: StatusType, // Only Success or Error
    pub message: String,
}

/// Manages status updates and WebSocket broadcasting.
/// Cheap to clone - all clones share the same underlying state.
#[derive(Clone)]
pub struct StatusManager {
    tx: broadcast::Sender<WebSocketMessage>,
    current_status: Arc<RwLock<Option<PersistentStatus>>>,
}

impl StatusManager {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel::<WebSocketMessage>(100);
        Self {
            tx,
            current_status: Arc::new(RwLock::new(None)),
        }
    }

    /// Update the status and broadcast to all connected WebSocket clients.
    pub async fn update(&self, status_type: StatusType, message: &str) {
        // Only store persistent states (Success clears errors, Error stores the error)
        let persistent_status = match status_type {
            StatusType::Success => None, // Clear any error state
            StatusType::Error => Some(PersistentStatus {
                status_type: StatusType::Error,
                message: message.to_string(),
            }),
            // Everything else just keeps the current state
            _ => {
                let status = self.current_status.read().await;
                status.clone() // Keep existing persistent state
            }
        };

        // Update the stored status
        {
            let mut status = self.current_status.write().await;
            *status = persistent_status;
        }

        // Send the message to all connected clients
        let _ = self.tx.send(WebSocketMessage {
            data: json!({
                "type": status_type.to_string(),
                "message": message
            })
            .to_string(),
        });
    }

    /// Subscribe to WebSocket messages (for new connections).
    pub fn subscribe(&self) -> broadcast::Receiver<WebSocketMessage> {
        self.tx.subscribe()
    }

    /// Get the current persistent status (for new connections).
    pub async fn get_current(&self) -> Option<PersistentStatus> {
        self.current_status.read().await.clone()
    }

    /// Send a raw WebSocket message (for initial errors, etc.).
    pub fn send_raw(&self, message: WebSocketMessage) {
        let _ = self.tx.send(message);
    }
}

impl Default for StatusManager {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
struct AppState {
    status_manager: StatusManager,
}

fn inject_live_reload_script(html_content: &str, socket_addr: SocketAddr, host: bool) -> String {
    let mut content = html_content.to_string();

    // Run cargo xtask build-cli-js to build the client.js file if missing
    let script_content = include_str!("../../js/dist/client.js").replace(
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

    content.push_str(&format!("\n\n<script>{script_content}</script>"));
    content
}

pub async fn start_dev_web_server(
    start_time: Instant,
    status_manager: StatusManager,
    host: bool,
    port: Option<u16>,
    initial_error: Option<String>,
) {
    // TODO: The dist dir should be configurable
    let dist_dir = "dist";

    // Send initial error if present
    if let Some(error) = initial_error {
        status_manager.send_raw(WebSocketMessage {
            data: json!({
                "type": StatusType::Error.to_string(),
                "message": error
            })
            .to_string(),
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

    // Use provided port or default to the constant PORT
    let starting_port = port.unwrap_or(PORT);

    let port = find_open_port(&addr, starting_port).await;
    let socket = TcpSocket::new_v4().unwrap();
    let _ = socket.set_reuseaddr(true);

    let socket_addr = SocketAddr::new(addr, port);
    socket.bind(socket_addr).unwrap();

    let listener = socket.listen(1024).unwrap();

    debug!("listening on {}", listener.local_addr().unwrap());

    let serve_dir =
        ServeDir::new(dist_dir).not_found_service(axum::routing::any(move || async move {
            handle_404(socket_addr, host, dist_dir).await
        }));

    // TODO: Return a `.well-known/appspecific/com.chrome.devtools.json` for Chrome

    let router = Router::new()
        .route("/ws", get(ws_handler))
        .fallback_service(serve_dir)
        .layer(middleware::from_fn(add_cache_headers))
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
            status_manager: status_manager.clone(),
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
        let original_headers = res.headers().clone();
        let body = res.into_body();
        let bytes = to_bytes(body, usize::MAX).await.unwrap();

        let body = String::from_utf8_lossy(&bytes).into_owned();

        let body_with_script = inject_live_reload_script(&body, socket_addr, host);
        let new_body_length = body_with_script.len();

        // Copy the headers from the original response
        let mut res = Response::new(body_with_script.into());
        *res.headers_mut() = original_headers;

        // Update Content-Length header to match new body size
        res.headers_mut().insert(
            CONTENT_LENGTH,
            HeaderValue::from_str(&new_body_length.to_string()).unwrap(),
        );

        res.extensions_mut().insert(uri);

        return res;
    }

    res
}

async fn add_cache_headers(req: Request, next: Next) -> Response {
    let uri = req.uri().clone();
    let mut res = next.run(req).await;

    if let Some(content_type) = res.headers().get(axum::http::header::CONTENT_TYPE) {
        let cache_header = cache_header_by_content(&uri, content_type);
        if let Some(cache_header) = cache_header {
            res.headers_mut()
                .insert(header::CACHE_CONTROL, cache_header);
        }
    }

    res
}

fn cache_header_by_content(uri: &Uri, content_type: &HeaderValue) -> Option<HeaderValue> {
    if content_type == HeaderValue::from_static("text/html") {
        // No cache for HTML files
        Some(HeaderValue::from_static(
            "no-cache, no-store, must-revalidate",
        ))
    }
    // If something comes from the assets path, assume that it's fingerprinted and can be cached for a long time
    // TODO: Same as dist, shouldn't be hardcoded
    else if uri.path().starts_with("/_maudit/") {
        Some(HeaderValue::from_static(
            "public, max-age=31536000, immutable",
        ))
    } else {
        // Don't try to cache anything else, the browser will decide based on the last-modified header
        None
    }
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    debug!("`{addr} connected.");
    // finalize the upgrade process by returning upgrade callback.
    // we can customize the callback by sending additional info such as address.
    ws.on_upgrade(move |socket| handle_socket(socket, addr, state.status_manager))
}

async fn handle_socket(
    socket: WebSocket,
    who: SocketAddr,
    status_manager: StatusManager,
) {
    let (mut sender, mut receiver) = socket.split();

    // Send current persistent status to new connection if there is one
    if let Some(persistent_status) = status_manager.get_current().await {
        let _ = sender
            .send(Message::Text(
                json!({
                    "type": persistent_status.status_type.to_string(),
                    "message": persistent_status.message
                })
                .to_string()
                .into(),
            ))
            .await;
    }

    let mut rx = status_manager.subscribe();

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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_status_manager_update_error_persists() {
        let manager = StatusManager::new();

        manager.update(StatusType::Error, "Something went wrong").await;

        let status = manager.get_current().await;
        assert!(status.is_some());
        let status = status.unwrap();
        assert!(matches!(status.status_type, StatusType::Error));
        assert_eq!(status.message, "Something went wrong");
    }

    #[tokio::test]
    async fn test_status_manager_update_success_clears_error() {
        let manager = StatusManager::new();

        // First set an error
        manager.update(StatusType::Error, "Build failed").await;
        assert!(manager.get_current().await.is_some());

        // Then send success - should clear the error
        manager.update(StatusType::Success, "Build succeeded").await;
        assert!(manager.get_current().await.is_none());
    }

    #[tokio::test]
    async fn test_status_manager_update_info_preserves_state() {
        let manager = StatusManager::new();

        // Set an error
        manager.update(StatusType::Error, "Build failed").await;
        let original_status = manager.get_current().await;
        assert!(original_status.is_some());

        // Send info - should preserve the error state
        manager.update(StatusType::Info, "Building...").await;
        let status = manager.get_current().await;
        assert!(status.is_some());
        assert_eq!(status.unwrap().message, "Build failed");
    }

    #[tokio::test]
    async fn test_status_manager_update_info_when_no_error() {
        let manager = StatusManager::new();

        // No prior state
        assert!(manager.get_current().await.is_none());

        // Send info - should remain None
        manager.update(StatusType::Info, "Building...").await;
        assert!(manager.get_current().await.is_none());
    }

    #[tokio::test]
    async fn test_status_manager_subscribe_receives_messages() {
        let manager = StatusManager::new();
        let mut rx = manager.subscribe();

        manager.update(StatusType::Info, "Hello").await;

        let msg = rx.try_recv();
        assert!(msg.is_ok());
        let msg = msg.unwrap();
        assert!(msg.data.contains("Hello"));
        assert!(msg.data.contains("info"));
    }

    #[tokio::test]
    async fn test_status_manager_multiple_subscribers() {
        let manager = StatusManager::new();
        let mut rx1 = manager.subscribe();
        let mut rx2 = manager.subscribe();

        manager.update(StatusType::Success, "Done").await;

        // Both subscribers should receive the message
        assert!(rx1.try_recv().is_ok());
        assert!(rx2.try_recv().is_ok());
    }

    #[tokio::test]
    async fn test_status_manager_send_raw() {
        let manager = StatusManager::new();
        let mut rx = manager.subscribe();

        manager.send_raw(WebSocketMessage {
            data: r#"{"custom": "message"}"#.to_string(),
        });

        let msg = rx.try_recv();
        assert!(msg.is_ok());
        assert_eq!(msg.unwrap().data, r#"{"custom": "message"}"#);
    }

    #[tokio::test]
    async fn test_status_manager_clone_shares_state() {
        let manager1 = StatusManager::new();
        let manager2 = manager1.clone();

        // Update via one clone
        manager1.update(StatusType::Error, "Error from clone 1").await;

        // Should be visible via the other clone
        let status = manager2.get_current().await;
        assert!(status.is_some());
        assert_eq!(status.unwrap().message, "Error from clone 1");
    }

    #[tokio::test]
    async fn test_status_manager_clone_shares_broadcast() {
        let manager1 = StatusManager::new();
        let manager2 = manager1.clone();

        // Subscribe via one clone
        let mut rx = manager2.subscribe();

        // Send via the other clone
        manager1.update(StatusType::Info, "From clone 1").await;

        // Should receive the message
        let msg = rx.try_recv();
        assert!(msg.is_ok());
        assert!(msg.unwrap().data.contains("From clone 1"));
    }
}
