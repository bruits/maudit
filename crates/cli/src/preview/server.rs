use std::path::PathBuf;

use axum::{handler::HandlerWithoutStateExt, http::StatusCode, Router};

use tower_http::{
    services::ServeDir,
    trace::{DefaultMakeSpan, TraceLayer},
};

pub async fn start_preview_web_server(dist_dir: PathBuf) {
    async fn handle_404() -> (StatusCode, &'static str) {
        (StatusCode::NOT_FOUND, "Not found")
    }

    let service = handle_404.into_service();
    let serve_dir = ServeDir::new(dist_dir).not_found_service(service);

    let router = Router::new().fallback_service(serve_dir).layer(
        TraceLayer::new_for_http().make_span_with(DefaultMakeSpan::default().include_headers(true)),
    );

    // run it with hyper
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    tracing::debug!("listening on {}", listener.local_addr().unwrap());

    axum::serve(listener, router.into_make_service())
        .await
        .unwrap();
}
