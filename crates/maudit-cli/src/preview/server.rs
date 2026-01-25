use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;

use axum::{
    Router,
    handler::HandlerWithoutStateExt,
    http::{StatusCode, header},
    response::IntoResponse,
};
use quanta::Instant;
use tokio::{fs, net::TcpSocket};
use tracing::{Level, debug};

use tower_http::{
    services::ServeDir,
    trace::{DefaultMakeSpan, TraceLayer},
};

use crate::consts::PORT;
use crate::server_utils::{CustomOnResponse, find_open_port, log_server_start};

pub async fn start_preview_web_server(dist_dir: PathBuf, host: bool) {
    let start_time = Instant::now();

    async fn handle_404(dist_dir: PathBuf) -> impl IntoResponse {
        let content = match fs::read_to_string(dist_dir.join("404.html")).await {
            Ok(custom_content) => custom_content,
            Err(_) => include_str!("../dev/404.html").to_string(),
        };

        (
            StatusCode::NOT_FOUND,
            [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
            content,
        )
            .into_response()
    }

    // run it with hyper, if --host 0.0.0.0 otherwise localhost
    let addr = if host {
        IpAddr::from([0, 0, 0, 0])
    } else {
        IpAddr::from([127, 0, 0, 1])
    };

    let port = find_open_port(&addr, PORT).await;
    let socket = TcpSocket::new_v4().unwrap();
    let _ = socket.set_reuseaddr(true);
    let _ = socket.set_reuseport(true);

    let socket_addr = SocketAddr::new(addr, port);
    socket.bind(socket_addr).unwrap();

    let listener = socket.listen(1024).unwrap();

    debug!("listening on {}", listener.local_addr().unwrap());

    let dist_dir_clone = dist_dir.clone();
    let service = (move || handle_404(dist_dir_clone.clone())).into_service();
    let serve_dir = ServeDir::new(dist_dir).not_found_service(service);

    let router = Router::new()
        .fallback_service(serve_dir)
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::default().include_headers(true)),
        )
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                .on_response(CustomOnResponse),
        );

    log_server_start(start_time, host, listener.local_addr().unwrap(), "Preview");

    axum::serve(listener, router.into_make_service())
        .await
        .unwrap();
}
