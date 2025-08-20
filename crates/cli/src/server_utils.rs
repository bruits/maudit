use axum::{body::Body, http::Uri, response::Response};
use colored::Colorize;
use local_ip_address::local_ip;
use std::{
    net::{IpAddr, SocketAddr},
    time::Duration,
};
use tokio::net::TcpSocket;
use tower_http::trace::OnResponse;
use tracing::{debug, info, Span};

use crate::logging::{format_elapsed_time, FormatElapsedTimeOptions};

pub fn log_server_start(
    start_time: std::time::Instant,
    host: bool,
    addr: SocketAddr,
    server_type: &str,
) {
    info!(name: "SKIP_FORMAT", "");
    let elapsed_time = format_elapsed_time(
        Ok(start_time.elapsed()),
        &FormatElapsedTimeOptions::default_dev(),
    )
    .unwrap();
    info!(name: "SKIP_FORMAT", "{} {}", "Maudit 👑".bold().bright_red(), format!("{} server started in {}", server_type, elapsed_time));
    info!(name: "SKIP_FORMAT", "");

    let port = addr.port();
    let url = format!("\x1b]8;;http://localhost:{port}\x1b\\http://localhost:{port}\x1b]8;;\x1b\\")
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
pub struct CustomOnResponse;

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

pub async fn find_open_port(address: &IpAddr, starting_port: u16) -> u16 {
    let mut port = starting_port;

    loop {
        let socket = TcpSocket::new_v4().unwrap();
        let socket_addr = SocketAddr::new(*address, port);
        match socket.bind(socket_addr) {
            Ok(_) => {
                debug!("Found open port: {}", port);
                return port;
            }
            Err(_) => {
                debug!(
                    "Port {} is already in use or failed to bind, trying next one",
                    port
                );
                port += 1;
            }
        }
    }
}
