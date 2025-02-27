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
use tokio::{net::TcpSocket, signal, sync::broadcast};
use tracing::{debug, info, Level, Span};

use std::{
    net::{IpAddr, SocketAddr},
    time::Duration,
};
use tower_http::{
    services::ServeDir,
    trace::{DefaultMakeSpan, OnResponse, TraceLayer},
};

use axum::extract::connect_info::ConnectInfo;
use futures::{stream::StreamExt, SinkExt};

use crate::logging::{format_elapsed_time, FormatElapsedTimeOptions};
use local_ip_address::local_ip;

#[derive(Clone, Debug)]
pub struct WebSocketMessage {
    pub data: String,
}

#[derive(Clone)]
struct AppState {
    tx: broadcast::Sender<WebSocketMessage>,
}

pub async fn start_dev_web_server(
    start_time: std::time::Instant,
    tx: broadcast::Sender<WebSocketMessage>,
    host: bool,
) {
    async fn handle_404() -> (StatusCode, &'static str) {
        (StatusCode::NOT_FOUND, "Not found")
    }

    let service = handle_404.into_service();
    let serve_dir = ServeDir::new("dist").not_found_service(service);

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
        .with_state(AppState { tx });

    log_server_start(start_time, host, listener.local_addr().unwrap());

    axum::serve(
        listener,
        router.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await
    .unwrap();
}

fn log_server_start(start_time: std::time::Instant, host: bool, addr: SocketAddr) {
    info!(name: "SKIP_FORMAT", "");
    let elasped_time = format_elapsed_time(
        Ok(start_time.elapsed()),
        &FormatElapsedTimeOptions::default_dev(),
    )
    .unwrap();
    info!(name: "SKIP_FORMAT", "{} {}", "Maudit 👑".bold().bright_red(), format!("{} {}", "server started in".dimmed(), elasped_time));
    info!(name: "SKIP_FORMAT", "");

    let port = addr.port();
    let url =
        format!("\x1b]8;;http://localhost:{port}\x1b\\http://localhost:{port}\x1b]8;;\x1b\\",)
            .bold()
            .underline()
            .bright_blue();
    let network_url = if host {
        let local_ip = local_ip().unwrap();
        format!("\x1b]8;;http://{local_ip}:{port}\x1b\\http://{local_ip}:{port}\x1b]8;;\x1b\\")
            .bold()
            .underline()
            .bright_magenta()
    } else {
        "Use --host to expose the server to your network".dimmed()
    };
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

        let mut body = String::from_utf8_lossy(&bytes).into_owned();

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

        // TODO: Handle HTML documents with no tags, e.g. `"Hello, world"`. Appending a raw script tag will cause it to show up as text.
        body.push_str(&format!("<script>{script_content}</script>"));

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

async fn find_open_port(address: &IpAddr, starting_port: u16) -> u16 {
    let mut port = starting_port;

    loop {
        let socket = TcpSocket::new_v4().unwrap();
        let socket_addr = SocketAddr::new(*address, port);
        match socket.bind(socket_addr) {
            Ok(_) => {
                debug!(name: "dev", "Found open port: {}", port);
                return port;
            }
            Err(_) => {
                debug!(name: "dev", "Port {} is already in use or failed to bind, trying next one", port);
                port += 1;
            }
        }
    }
}
