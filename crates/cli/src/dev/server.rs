use axum::{
    body::to_bytes,
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Request,
    },
    handler::HandlerWithoutStateExt,
    http::{HeaderValue, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::any,
    Router,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use std::net::SocketAddr;
use tower_http::{
    services::ServeDir,
    trace::{DefaultMakeSpan, TraceLayer},
};

use axum::extract::connect_info::ConnectInfo;
use axum::extract::ws::CloseFrame;
use futures::stream::StreamExt;

pub async fn start_web_server() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                format!("{}=debug,tower_http=debug", env!("CARGO_CRATE_NAME")).into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    async fn handle_404() -> (StatusCode, &'static str) {
        (StatusCode::NOT_FOUND, "Not found")
    }

    let service = handle_404.into_service();
    let serve_dir = ServeDir::new("dist").not_found_service(service);

    let router = Router::new()
        .route("/ws", any(ws_handler))
        .fallback_service(serve_dir)
        .layer(middleware::from_fn(add_dev_client_script))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::default().include_headers(true)),
        );

    // run it with hyper
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    tracing::debug!("listening on {}", listener.local_addr().unwrap());
    axum::serve(
        listener,
        router.into_make_service_with_connect_info::<SocketAddr>(),
    )
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

        let body = String::from_utf8_lossy(&bytes).replace(
            "</body></html>",
            r#"<script>
        const socket = new WebSocket('ws://localhost:3000/ws');

        socket.addEventListener('open', function (event) {
            socket.send('Hello Server!');
        });

        socket.addEventListener('message', function (event) {
            console.log('Message from server ', event.data);
        });
        </script></body></html>"#,
        );

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
) -> impl IntoResponse {
    println!("`{addr} connected.");
    // finalize the upgrade process by returning upgrade callback.
    // we can customize the callback by sending additional info such as address.
    ws.on_upgrade(move |socket| handle_socket(socket, addr))
}

async fn handle_socket(socket: WebSocket, who: SocketAddr) {
    let (_, mut receiver) = socket.split();

    while let Some(Ok(msg)) = receiver.next().await {
        if let Message::Close(Some(CloseFrame { code, reason })) = &msg {
            println!("Client {who} sent close with code {code} and reason `{reason}`");
            break;
        } else if let Message::Close(None) = &msg {
            println!("Client {who} sent close without a CloseFrame");
            break;
        }

        match msg {
            Message::Text(t) => {
                println!("<<< {who} sent str: {t:?}");
            }
            Message::Binary(d) => {
                println!("<<< {who} sent {} bytes: {d:?}", d.len());
            }
            _ => {}
        }
    }

    // returning from the handler closes the websocket connection
    println!("Websocket context {who} destroyed");
}
