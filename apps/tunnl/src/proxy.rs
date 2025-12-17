//! HTTP proxy layer for forwarding traffic through SSH tunnels.

use std::convert::Infallible;
use std::sync::Arc;

use bytes::Bytes;
use http_body_util::Full;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use log::{debug, error, info, warn};
use tokio::net::TcpListener;

use crate::state::AppState;

/// Extract subdomain from Host header.
/// Supports: "tunnel-xxx.localhost:8080" or "tunnel-xxx.example.com"
fn extract_subdomain(host: &str) -> Option<String> {
    let host_without_port = host.split(':').next()?;
    let subdomain = host_without_port.split('.').next()?;
    
    if subdomain.starts_with("tunnel-") {
        Some(subdomain.to_string())
    } else {
        None
    }
}

/// Handle an incoming HTTP request by forwarding it through the SSH tunnel.
async fn handle_http_request(
    req: Request<hyper::body::Incoming>,
    state: Arc<AppState>,
) -> Result<Response<Full<Bytes>>, Infallible> {
    let host = req
        .headers()
        .get("host")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");
    
    let subdomain = match extract_subdomain(host) {
        Some(s) => s,
        None => {
            let tunnels = state.list_tunnels().await;
            let tunnel_list: Vec<String> = tunnels.iter()
                .map(|t| format!("  - http://{}.localhost:8080", t.subdomain))
                .collect();
            
            let body = if tunnel_list.is_empty() {
                "No tunnels registered.\n\nConnect with: ssh -N -R 80:localhost:PORT -p 2222 user@server".to_string()
            } else {
                format!(
                    "Available tunnels:\n{}\n\nUse: curl -H \"Host: SUBDOMAIN.localhost\" http://localhost:8080/",
                    tunnel_list.join("\n")
                )
            };
            
            return Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Full::new(Bytes::from(body)))
                .unwrap());
        }
    };

    info!("HTTP request for subdomain: {}", subdomain);

    let tunnel = match state.get_tunnel(&subdomain).await {
        Some(t) => t,
        None => {
            return Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Full::new(Bytes::from(format!("Tunnel '{}' not found", subdomain))))
                .unwrap());
        }
    };

    info!(
        "Forwarding to tunnel: {} -> localhost:{}",
        subdomain, tunnel.requested_port
    );

    let channel_result = tunnel
        .handle
        .channel_open_forwarded_tcpip(
            "localhost",
            tunnel.server_port,
            "127.0.0.1",
            12345,
        )
        .await;

    let mut channel = match channel_result {
        Ok(ch) => ch,
        Err(e) => {
            error!("Failed to open forwarded channel: {:?}", e);
            return Ok(Response::builder()
                .status(StatusCode::BAD_GATEWAY)
                .body(Full::new(Bytes::from(format!("Failed to connect to tunnel: {:?}", e))))
                .unwrap());
        }
    };

    info!("Opened forwarded channel to client");

    let method = req.method().clone();
    let uri = req.uri().clone();
    let path = uri.path_and_query().map(|p| p.to_string()).unwrap_or_else(|| "/".to_string());
    
    let http_request = format!(
        "{} {} HTTP/1.1\r\nHost: localhost:{}\r\nConnection: close\r\n\r\n",
        method, path, tunnel.requested_port
    );

    if let Err(e) = channel.data(http_request.as_bytes()).await {
        error!("Failed to send data through channel: {:?}", e);
        return Ok(Response::builder()
            .status(StatusCode::BAD_GATEWAY)
            .body(Full::new(Bytes::from("Failed to send request through tunnel")))
            .unwrap());
    }

    channel.eof().await.ok();

    let mut response_data = Vec::new();
    let timeout = tokio::time::timeout(std::time::Duration::from_secs(30), async {
        loop {
            match channel.wait().await {
                Some(msg) => {
                    match msg {
                        russh::ChannelMsg::Data { data } => {
                            response_data.extend_from_slice(&data);
                        }
                        russh::ChannelMsg::Eof | russh::ChannelMsg::Close => {
                            break;
                        }
                        _ => {}
                    }
                }
                None => break,
            }
        }
    });

    if timeout.await.is_err() {
        warn!("Timeout waiting for response from tunnel");
        return Ok(Response::builder()
            .status(StatusCode::GATEWAY_TIMEOUT)
            .body(Full::new(Bytes::from("Timeout waiting for response")))
            .unwrap());
    }

    info!("Received {} bytes from tunnel", response_data.len());

    let response_str = String::from_utf8_lossy(&response_data);
    
    if let Some(body_start) = response_str.find("\r\n\r\n") {
        let body = &response_data[body_start + 4..];
        
        let status = if response_str.starts_with("HTTP/1.") {
            let status_line = response_str.lines().next().unwrap_or("");
            status_line.split_whitespace().nth(1)
                .and_then(|s| s.parse().ok())
                .unwrap_or(200)
        } else {
            200
        };

        Ok(Response::builder()
            .status(StatusCode::from_u16(status).unwrap_or(StatusCode::OK))
            .body(Full::new(Bytes::from(body.to_vec())))
            .unwrap())
    } else {
        Ok(Response::builder()
            .status(StatusCode::OK)
            .body(Full::new(Bytes::from(response_data)))
            .unwrap())
    }
}

/// Run the HTTP proxy server.
pub async fn run_http_proxy(state: Arc<AppState>, addr: &str) -> anyhow::Result<()> {
    let listener = TcpListener::bind(addr).await?;
    info!("HTTP proxy listening on {}", addr);

    loop {
        let (stream, remote_addr) = listener.accept().await?;
        let io = TokioIo::new(stream);
        let state = state.clone();

        tokio::spawn(async move {
            debug!("HTTP connection from {}", remote_addr);
            
            let service = service_fn(move |req| {
                let state = state.clone();
                handle_http_request(req, state)
            });

            if let Err(e) = http1::Builder::new()
                .serve_connection(io, service)
                .await
            {
                error!("HTTP connection error: {:?}", e);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_subdomain() {
        assert_eq!(
            extract_subdomain("tunnel-abc123.localhost:8080"),
            Some("tunnel-abc123".to_string())
        );
        assert_eq!(
            extract_subdomain("tunnel-xyz.example.com"),
            Some("tunnel-xyz".to_string())
        );
        assert_eq!(extract_subdomain("localhost:8080"), None);
        assert_eq!(extract_subdomain("example.com"), None);
    }
}
