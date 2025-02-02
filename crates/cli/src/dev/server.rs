use axum::{
    body::{to_bytes, Body},
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Request, State,
    },
    handler::HandlerWithoutStateExt,
    http::{HeaderValue, StatusCode, Uri},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use colored::Colorize;
use tokio::{signal, sync::broadcast};
use tracing::{debug, info, Level, Span};

use std::{net::SocketAddr, time::Duration};
use tower_http::{
    services::ServeDir,
    trace::{DefaultMakeSpan, OnResponse, TraceLayer},
};

use axum::extract::connect_info::ConnectInfo;
use futures::{stream::StreamExt, SinkExt};

use crate::logging::{format_elapsed_time, FormatElapsedTimeOptions};

#[derive(Clone, Debug)]
pub struct WebSocketMessage {
    pub data: String,
}

#[derive(Clone)]
struct AppState {
    tx: broadcast::Sender<WebSocketMessage>,
}

pub async fn start_dev_web_server(tx: broadcast::Sender<WebSocketMessage>) {
    let start_time = std::time::Instant::now();
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
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                .on_response(CustomOnResponse),
        )
        .with_state(AppState { tx });

    // run it with hyper, if --host 0.0.0.0 otherwise localhost
    let addr = if std::env::args().any(|arg| arg == "--host") {
        "0.0.0.0"
    } else {
        "localhost"
    };
    let port = 3000;
    let listener = tokio::net::TcpListener::bind(format!("{}:{}", addr, port))
        .await
        .unwrap();

    debug!("listening on {}", listener.local_addr().unwrap());

    log_server_start(start_time);

    axum::serve(
        listener,
        router.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await
    .unwrap();
}

fn log_server_start(start_time: std::time::Instant) {
    info!(name: "SKIP_FORMAT", "");
    let elasped_time = format_elapsed_time(Ok(start_time.elapsed()), &Default::default()).unwrap();
    info!(name: "SKIP_FORMAT", "{} {}", "Maudit 👑".bold().bright_red(), format!("{} {}", "server started in".dimmed(), elasped_time));
    info!(name: "SKIP_FORMAT", "");

    let url = "\x1b]8;;http://localhost:3000\x1b\\http://localhost:3000\x1b]8;;\x1b\\"
        .bold()
        .underline()
        .bright_blue();
    let network_url = "\x1b]8;;http://192.168.0.1:3000\x1b\\http://192.168.0.1:3000\x1b]8;;\x1b\\"
        .bold()
        .underline()
        .bright_magenta();
    info!(name: "SKIP_FORMAT", "🮔  {}    {}", "Local".bold(), url);
    info!(name: "SKIP_FORMAT", "🮔  {}  {}", "Network".bold(), network_url);
    info!(name: "SKIP_FORMAT", "");

    info!(name: "server", "{}", "waiting for requests...".dimmed());
}

#[derive(Clone, Debug)]
struct CustomOnResponse;

impl OnResponse<Body> for CustomOnResponse {
    fn on_response(self, response: &Response<Body>, latency: Duration, _span: &Span) {
        let status = response.status();

        // Skip informational responses
        if status.is_informational() {
            return;
        }

        let status = if status.is_server_error() {
            status.to_string().red()
        } else if status.is_client_error() {
            status.to_string().yellow()
        } else {
            status.to_string().green()
        };

        // There's allegedly a way to get the request URI from the span, but I can't figure it out
        let uri = response
            .extensions()
            .get::<Uri>()
            .unwrap_or(&Uri::default())
            .to_string()
            .bold();

        let latency =
            format_elapsed_time(Ok(latency), &FormatElapsedTimeOptions::default()).unwrap();

        let message = format!("{} {} {}", status, uri, latency);

        info!(name: "", "{}", message);
    }
}

async fn add_dev_client_script(req: Request, next: Next) -> Response {
    let uri = req.uri().clone();
    let mut res: axum::http::Response<Body> = next.run(req).await;

    res.extensions_mut().insert(uri.clone());

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
