//! HTTP proxy layer for forwarding traffic through SSH tunnels.
//! Uses TCP passthrough with Host header peek for subdomain routing.

use std::sync::Arc;

use log::{debug, error, info, warn};
use tokio::io::{AsyncWriteExt, copy_bidirectional};
use tokio::net::{TcpListener, TcpStream};

use crate::config::{get as get_config, get_tunnel_url};
use crate::state::AppState;

/// Extract subdomain from Host header based on a given base domain.
/// e.g., base_domain="localhost", host="test.localhost:8080" -> "test"
/// e.g., base_domain="example.com", host="test.example.com" -> "test"
/// 
/// Validates subdomain length (max 63 chars) and characters (alphanumeric + hyphen).
fn extract_subdomain_with_base(host: &str, base_domain: &str) -> Option<String> {
    // Host header might have port, remove it for comparison
    let host_without_port = host.split(':').next().unwrap_or(host);
    
    // Check if host ends with ".base_domain" (e.g., "test.localhost" ends with ".localhost")
    let suffix = format!(".{}", base_domain);
    if host_without_port.ends_with(&suffix) {
        // Extract subdomain (everything before the suffix)
        let subdomain = &host_without_port[..host_without_port.len() - suffix.len()];
        
        // Validate: not empty, no dots (single-level subdomain only)
        if subdomain.is_empty() || subdomain.contains('.') {
            return None;
        }
        
        // Validate length (DNS label limit is 63 characters)
        if subdomain.len() > 63 {
            warn!("Subdomain too long (max 63 chars): {} chars", subdomain.len());
            return None;
        }
        
        // Validate characters (alphanumeric and hyphens only, case-insensitive)
        let subdomain_lower = subdomain.to_lowercase();
        if !subdomain_lower.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
            warn!("Subdomain contains invalid characters: {}", subdomain);
            return None;
        }
        
        // Cannot start or end with hyphen
        if subdomain_lower.starts_with('-') || subdomain_lower.ends_with('-') {
            warn!("Subdomain cannot start or end with hyphen: {}", subdomain);
            return None;
        }
        
        return Some(subdomain_lower);
    }
    
    None
}

/// Extract subdomain from Host header based on TUNNEL_URL configuration.
/// If TUNNEL_URL is "localhost:8080", then "test.localhost:8080" -> "test"
/// If TUNNEL_URL is "example.com", then "test.example.com" -> "test"
fn extract_subdomain(host: &str) -> Option<String> {
    let tunnel_url = &get_config().tunnel_url;
    
    // Remove port from tunnel_url for comparison (e.g., "localhost:8080" -> "localhost")
    let base_domain = tunnel_url.split(':').next().unwrap_or(tunnel_url);
    
    extract_subdomain_with_base(host, base_domain)
}

/// Extract Host header value from raw HTTP request bytes.
fn extract_host_from_raw(data: &[u8]) -> Option<String> {
    let text = std::str::from_utf8(data).ok()?;

    for line in text.lines() {
        let lower = line.to_lowercase();
        if lower.starts_with("host:") {
            return Some(line[5..].trim().to_string());
        }
        // Empty line means end of headers
        if line.is_empty() {
            break;
        }
    }
    None
}

/// Generate error response HTML.
fn error_response(status: u16, message: &str) -> Vec<u8> {
    let body = message.as_bytes();
    format!(
        "HTTP/1.1 {} {}\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        status,
        match status {
            400 => "Bad Request",
            404 => "Not Found",
            502 => "Bad Gateway",
            504 => "Gateway Timeout",
            _ => "Error",
        },
        body.len(),
        message
    )
    .into_bytes()
}

/// Generate tunnel list response.
fn tunnel_list_response() -> Vec<u8> {
    let tunnel_url = &get_config().tunnel_url;

    let body = format!(
        "Tunnel Proxy Server\n\nUse: curl -H \"Host: SUBDOMAIN.{}\" <address>\n\nConnect with: ssh -R 8000:localhost:8000 -p 2222 <subdomain>@server",
        tunnel_url
    );

    error_response(400, &body)
}

/// Handle a single TCP connection with peek-based routing.
async fn handle_connection(mut stream: TcpStream, state: Arc<AppState>) {
    // Peek at the first bytes to extract Host header
    let mut peek_buf = [0u8; 2048];
    let n = match stream.peek(&mut peek_buf).await {
        Ok(0) => {
            debug!("Connection closed before data received");
            return;
        }
        Ok(n) => n,
        Err(e) => {
            error!("Failed to peek data: {:?}", e);
            return;
        }
    };

    // Extract Host header from peeked data
    let host = match extract_host_from_raw(&peek_buf[..n]) {
        Some(h) => h,
        None => {
            warn!("No Host header found in request");
            let response = tunnel_list_response();
            let _ = stream.write_all(&response).await;
            return;
        }
    };

    // Extract subdomain from Host
    let subdomain = match extract_subdomain(&host) {
        Some(s) => s,
        None => {
            // No valid subdomain, show available tunnels
            let tunnels = state.list_tunnels().await;
            let tunnel_url = &get_config().tunnel_url;
            let tunnel_list: Vec<String> = tunnels
                .iter()
                .map(|t| format!("  - {}", get_tunnel_url(&t.subdomain)))
                .collect();

            let body = if tunnel_list.is_empty() {
                "No tunnels registered.\n\nConnect with: ssh -R 8000:localhost:8000 -p 2222 <subdomain>@server".to_string()
            } else {
                format!(
                    "Available tunnels:\n{}\n\nUse: curl -H \"Host: SUBDOMAIN.{}\" <address>",
                    tunnel_list.join("\n"),
                    tunnel_url
                )
            };

            let response = error_response(400, &body);
            let _ = stream.write_all(&response).await;
            return;
        }
    };

    info!("HTTP request for subdomain: {}", subdomain);

    // Look up tunnel
    let tunnel = match state.get_tunnel(&subdomain).await {
        Some(t) => t,
        None => {
            let response = error_response(404, &format!("Tunnel '{}' not found", subdomain));
            let _ = stream.write_all(&response).await;
            return;
        }
    };

    info!(
        "Forwarding to tunnel: {} -> localhost:{}",
        subdomain, tunnel.requested_port
    );

    // Open SSH forwarded channel
    let channel_result = tunnel
        .handle
        .channel_open_forwarded_tcpip(
            &tunnel.requested_address,
            tunnel.requested_port,
            "127.0.0.1",
            stream.peer_addr().map(|a| a.port() as u32).unwrap_or(0),
        )
        .await;

    let channel = match channel_result {
        Ok(ch) => ch,
        Err(e) => {
            error!("Failed to open forwarded channel: {:?}", e);
            let response = error_response(502, &format!("Failed to connect to tunnel: {:?}", e));
            let _ = stream.write_all(&response).await;
            return;
        }
    };

    info!("Opened forwarded channel to client");

    // Convert SSH channel to stream for bidirectional I/O
    let mut channel_stream = channel.into_stream();

    // Bidirectional copy between TCP stream and SSH channel stream
    let timeout = tokio::time::Duration::from_secs(300); // 5 minute timeout
    let result = tokio::time::timeout(timeout, async {
        copy_bidirectional(&mut stream, &mut channel_stream).await
    })
    .await;

    match result {
        Ok(Ok((to_ssh, to_tcp))) => {
            info!(
                "[{}] Connection completed: {} bytes to SSH, {} bytes to TCP",
                subdomain, to_ssh, to_tcp
            );
        }
        Ok(Err(e)) => {
            debug!("[{}] Copy error (may be normal on close): {:?}", subdomain, e);
        }
        Err(_) => {
            warn!("[{}] Connection timeout after 5 minutes", subdomain);
        }
    }
}

/// Run the HTTP proxy server.
pub async fn run_http_proxy(state: Arc<AppState>, addr: &str) -> anyhow::Result<()> {
    let listener = TcpListener::bind(addr).await?;
    info!("HTTP proxy listening on {}", addr);

    loop {
        let (stream, remote_addr) = listener.accept().await?;
        let state = state.clone();

        tokio::spawn(async move {
            debug!("HTTP connection from {}", remote_addr);
            handle_connection(stream, state).await;
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_subdomain_with_localhost() {
        // With base_domain = "localhost"
        assert_eq!(
            extract_subdomain_with_base("test.localhost:8080", "localhost"),
            Some("test".to_string())
        );
        assert_eq!(
            extract_subdomain_with_base("tunnel-abc123.localhost:8080", "localhost"),
            Some("tunnel-abc123".to_string())
        );
        assert_eq!(
            extract_subdomain_with_base("myapp.localhost", "localhost"),
            Some("myapp".to_string())
        );
        // No subdomain
        assert_eq!(extract_subdomain_with_base("localhost:8080", "localhost"), None);
        assert_eq!(extract_subdomain_with_base("localhost", "localhost"), None);
    }

    #[test]
    fn test_extract_subdomain_with_domain() {
        // With base_domain = "example.com"
        assert_eq!(
            extract_subdomain_with_base("test.example.com", "example.com"),
            Some("test".to_string())
        );
        assert_eq!(
            extract_subdomain_with_base("tunnel-xyz.example.com:8080", "example.com"),
            Some("tunnel-xyz".to_string())
        );
        // No subdomain
        assert_eq!(extract_subdomain_with_base("example.com", "example.com"), None);
        assert_eq!(extract_subdomain_with_base("example.com:8080", "example.com"), None);
        // Different domain should not match
        assert_eq!(extract_subdomain_with_base("test.other.com", "example.com"), None);
    }

    #[test]
    fn test_extract_subdomain_rejects_nested() {
        // Should reject nested subdomains (e.g., "a.b.localhost")
        assert_eq!(extract_subdomain_with_base("a.b.localhost", "localhost"), None);
        assert_eq!(extract_subdomain_with_base("sub.test.example.com", "example.com"), None);
    }

    #[test]
    fn test_extract_subdomain_with_base_domain_containing_port() {
        // When TUNNEL_URL is "localhost:8080", the base_domain passed should be "localhost"
        // This tests that the port stripping logic works correctly
        
        // Host with same port as TUNNEL_URL
        assert_eq!(
            extract_subdomain_with_base("myapp.localhost:8080", "localhost"),
            Some("myapp".to_string())
        );
        
        // Host with different port (should still work, we only care about domain)
        assert_eq!(
            extract_subdomain_with_base("myapp.localhost:9000", "localhost"),
            Some("myapp".to_string())
        );
        
        // Host without port
        assert_eq!(
            extract_subdomain_with_base("myapp.localhost", "localhost"),
            Some("myapp".to_string())
        );
        
        // Base domain itself (no subdomain)
        assert_eq!(extract_subdomain_with_base("localhost:8080", "localhost"), None);
        
        // Test with multi-level domain like "tunnel.example.com"
        assert_eq!(
            extract_subdomain_with_base("myapp.tunnel.example.com:8080", "tunnel.example.com"),
            Some("myapp".to_string())
        );
        assert_eq!(
            extract_subdomain_with_base("tunnel.example.com:8080", "tunnel.example.com"),
            None
        );
    }

    #[test]
    fn test_extract_host_from_raw() {
        let request = b"GET / HTTP/1.1\r\nHost: tunnel-abc.localhost:8080\r\nUser-Agent: curl\r\n\r\n";
        assert_eq!(
            extract_host_from_raw(request),
            Some("tunnel-abc.localhost:8080".to_string())
        );

        let request_lower = b"GET / HTTP/1.1\r\nhost: tunnel-xyz.example.com\r\n\r\n";
        assert_eq!(
            extract_host_from_raw(request_lower),
            Some("tunnel-xyz.example.com".to_string())
        );

        let no_host = b"GET / HTTP/1.1\r\nUser-Agent: curl\r\n\r\n";
        assert_eq!(extract_host_from_raw(no_host), None);
    }
}
