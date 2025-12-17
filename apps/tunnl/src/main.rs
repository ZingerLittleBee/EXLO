//! SSH Reverse Tunnel Server with HTTP Proxy
//!
//! A high-performance SSH server that handles reverse port forwarding requests
//! using "virtual bind" and an HTTP proxy layer that routes traffic through
//! SSH tunnels to connected clients.
//!
//! ## Usage
//! ```bash
//! # Start the server
//! RUST_LOG=info cargo run
//!
//! # Connect SSH tunnel (in another terminal)
//! ssh -o StrictHostKeyChecking=no -R 80:localhost:3000 -p 2222 test@localhost
//!
//! # Access via HTTP proxy (server prints the subdomain)
//! curl -H "Host: tunnel-xxx.localhost" http://localhost:8080/
//! ```

use std::collections::HashMap;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use log::{debug, error, info, warn};
use russh::keys::PublicKey;
use russh::server::{Auth, Handle, Handler, Msg, Server, Session};
use russh::{Channel, ChannelId};
use russh_keys::HashAlg;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::RwLock;

// =============================================================================
// Error Types (using thiserror for domain-specific errors)
// =============================================================================

/// Custom error types for tunnel-related operations.
#[derive(Debug, thiserror::Error)]
pub enum TunnelError {
    #[error("Authentication failed: {0}")]
    AuthFailed(String),

    #[error("Subdomain '{0}' is already taken")]
    SubdomainTaken(String),

    #[error("Tunnel not found for subdomain '{0}'")]
    TunnelNotFound(String),

    #[error("SSH protocol error: {0}")]
    SshError(#[from] russh::Error),

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
}

// =============================================================================
// State Types
// =============================================================================

/// Information about a registered tunnel, including the SSH session handle
/// for forwarding traffic.
#[derive(Debug, Clone)]
pub struct TunnelInfo {
    /// The assigned subdomain (e.g., "abc123")
    pub subdomain: String,
    /// SSH session handle for opening forwarded channels
    pub handle: Handle,
    /// The address the client requested to forward
    pub requested_address: String,
    /// The port the client requested (client's localhost port)
    pub requested_port: u32,
    /// Server port that was "virtually" bound
    pub server_port: u32,
    /// When this tunnel was created
    pub created_at: Instant,
    /// The client's username
    pub username: String,
}

/// Thread-safe global state for the tunnel registry.
#[derive(Debug, Default)]
pub struct AppState {
    /// Map from subdomain -> TunnelInfo
    pub tunnels: RwLock<HashMap<String, TunnelInfo>>,
}

impl AppState {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn register_tunnel(&self, info: TunnelInfo) -> Result<(), TunnelError> {
        let mut tunnels = self.tunnels.write().await;
        if tunnels.contains_key(&info.subdomain) {
            return Err(TunnelError::SubdomainTaken(info.subdomain));
        }
        info!("Registered tunnel: {} -> localhost:{}", info.subdomain, info.requested_port);
        tunnels.insert(info.subdomain.clone(), info);
        Ok(())
    }

    pub async fn remove_tunnel(&self, subdomain: &str) -> Result<TunnelInfo, TunnelError> {
        let mut tunnels = self.tunnels.write().await;
        tunnels
            .remove(subdomain)
            .ok_or_else(|| TunnelError::TunnelNotFound(subdomain.to_string()))
    }

    pub async fn get_tunnel(&self, subdomain: &str) -> Option<TunnelInfo> {
        let tunnels = self.tunnels.read().await;
        tunnels.get(subdomain).cloned()
    }

    pub async fn list_tunnels(&self) -> Vec<TunnelInfo> {
        let tunnels = self.tunnels.read().await;
        tunnels.values().cloned().collect()
    }
}

// =============================================================================
// SSH Server Implementation
// =============================================================================

#[derive(Clone)]
pub struct TunnelServer {
    state: Arc<AppState>,
}

impl TunnelServer {
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }
}

impl Server for TunnelServer {
    type Handler = SshHandler;

    fn new_client(&mut self, peer_addr: Option<SocketAddr>) -> Self::Handler {
        info!("New SSH connection from {:?}", peer_addr);
        SshHandler::new(self.state.clone(), peer_addr)
    }

    fn handle_session_error(&mut self, error: <Self::Handler as Handler>::Error) {
        error!("Session error: {:?}", error);
    }
}

// =============================================================================
// SSH Handler Implementation
// =============================================================================

pub struct SshHandler {
    state: Arc<AppState>,
    peer_addr: Option<SocketAddr>,
    username: Option<String>,
    /// SSH session handle, captured after authentication
    session_handle: Option<Handle>,
    registered_subdomains: Vec<String>,
    subdomain_counter: u32,
}

impl SshHandler {
    pub fn new(state: Arc<AppState>, peer_addr: Option<SocketAddr>) -> Self {
        Self {
            state,
            peer_addr,
            username: None,
            session_handle: None,
            registered_subdomains: Vec::new(),
            subdomain_counter: 0,
        }
    }

    fn generate_subdomain(&mut self) -> String {
        self.subdomain_counter += 1;
        let random_part: u32 = rand_simple();
        format!("tunnel-{:06x}-{}", random_part, self.subdomain_counter)
    }
}

fn rand_simple() -> u32 {
    use std::time::SystemTime;
    let duration = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    ((duration.as_nanos() as u64 ^ 0x5DEECE66D) & 0xFFFFFF) as u32
}

#[async_trait]
impl Handler for SshHandler {
    type Error = TunnelError;

    /// Called when authentication succeeds - capture the session handle
    async fn auth_succeeded(&mut self, session: &mut Session) -> Result<(), Self::Error> {
        info!("Authentication succeeded for user: {:?}", self.username);
        // Capture the session handle for later use in forwarding
        self.session_handle = Some(session.handle());
        Ok(())
    }

    async fn channel_close(
        &mut self,
        channel: ChannelId,
        _session: &mut Session,
    ) -> Result<(), Self::Error> {
        info!("Channel {:?} closed, cleaning up tunnels...", channel);
        
        for subdomain in &self.registered_subdomains {
            match self.state.remove_tunnel(subdomain).await {
                Ok(_) => info!("Removed tunnel: {}", subdomain),
                Err(e) => warn!("Failed to remove tunnel {}: {}", subdomain, e),
            }
        }
        self.registered_subdomains.clear();
        
        Ok(())
    }

    async fn auth_publickey(
        &mut self,
        user: &str,
        public_key: &PublicKey,
    ) -> Result<Auth, Self::Error> {
        let fingerprint = public_key.fingerprint(HashAlg::Sha256);
        
        info!(
            "Public key auth attempt: user='{}', fingerprint='{}'",
            user, fingerprint
        );
        
        self.username = Some(user.to_string());
        Ok(Auth::Accept)
    }

    /// Handle reverse port forwarding request with Virtual Bind
    async fn tcpip_forward(
        &mut self,
        address: &str,
        port: &mut u32,
        _session: &mut Session,
    ) -> Result<bool, Self::Error> {
        info!(
            "=== Virtual Tunnel Request ===\n\
             Address: '{}'\n\
             Port: {}\n\
             User: {:?}\n\
             Peer: {:?}",
            address, port, self.username, self.peer_addr
        );

        // Get the session handle (should be set after auth_succeeded)
        let handle = match &self.session_handle {
            Some(h) => h.clone(),
            None => {
                error!("No session handle available!");
                return Ok(false);
            }
        };

        let subdomain = self.generate_subdomain();
        let username = self.username.clone().unwrap_or_else(|| "anonymous".to_string());

        let tunnel_info = TunnelInfo {
            subdomain: subdomain.clone(),
            handle,
            requested_address: address.to_string(),
            requested_port: *port,
            server_port: 80, // The server port (virtual)
            created_at: Instant::now(),
            username,
        };

        match self.state.register_tunnel(tunnel_info).await {
            Ok(()) => {
                info!(
                    "✓ Virtual tunnel registered!\n\
                     Subdomain: {}\n\
                     Access URL: http://{}.localhost:8080",
                    subdomain, subdomain
                );
                self.registered_subdomains.push(subdomain);
                Ok(true)
            }
            Err(TunnelError::SubdomainTaken(s)) => {
                warn!("Subdomain {} already taken", s);
                Ok(false)
            }
            Err(e) => {
                error!("Failed to register tunnel: {}", e);
                Err(e)
            }
        }
    }

    async fn cancel_tcpip_forward(
        &mut self,
        address: &str,
        port: u32,
        _session: &mut Session,
    ) -> Result<bool, Self::Error> {
        info!("Cancel tcpip_forward: address='{}', port={}", address, port);

        let tunnels_to_remove: Vec<String> = self.registered_subdomains.clone();
        
        for subdomain in tunnels_to_remove {
            if let Ok(info) = self.state.remove_tunnel(&subdomain).await {
                if info.requested_address == address && info.requested_port == port {
                    self.registered_subdomains.retain(|s| s != &subdomain);
                    info!("Removed tunnel: {}", subdomain);
                }
            }
        }

        Ok(true)
    }

    async fn channel_open_session(
        &mut self,
        channel: Channel<Msg>,
        _session: &mut Session,
    ) -> Result<bool, Self::Error> {
        info!("Session channel opened: id={:?}", channel.id());
        Ok(true)
    }

    async fn data(
        &mut self,
        channel: ChannelId,
        data: &[u8],
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        debug!("Data received on channel {:?}: {} bytes", channel, data.len());
        session.data(channel, data.to_vec().into())?;
        Ok(())
    }

    async fn channel_eof(
        &mut self,
        channel: ChannelId,
        _session: &mut Session,
    ) -> Result<(), Self::Error> {
        debug!("EOF on channel {:?}", channel);
        Ok(())
    }
}

// =============================================================================
// HTTP Proxy Implementation
// =============================================================================

/// Extract subdomain from Host header
/// Supports: "tunnel-xxx.localhost:8080" or "tunnel-xxx.example.com"
fn extract_subdomain(host: &str) -> Option<String> {
    // Remove port if present
    let host_without_port = host.split(':').next()?;
    
    // Get the first part before any dots
    let subdomain = host_without_port.split('.').next()?;
    
    if subdomain.starts_with("tunnel-") {
        Some(subdomain.to_string())
    } else {
        None
    }
}

/// Handle an incoming HTTP request by forwarding it through the SSH tunnel
async fn handle_http_request(
    req: Request<hyper::body::Incoming>,
    state: Arc<AppState>,
) -> Result<Response<Full<Bytes>>, Infallible> {
    // Extract subdomain from Host header
    let host = req
        .headers()
        .get("host")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");
    
    let subdomain = match extract_subdomain(host) {
        Some(s) => s,
        None => {
            // List available tunnels if no subdomain specified
            let tunnels = state.list_tunnels().await;
            let tunnel_list: Vec<String> = tunnels.iter()
                .map(|t| format!("  - http://{}.localhost:8080", t.subdomain))
                .collect();
            
            let body = if tunnel_list.is_empty() {
                "No tunnels registered.\n\nConnect with: ssh -R 80:localhost:PORT -p 2222 user@server".to_string()
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

    // Look up the tunnel
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

    // Open a forwarded channel to the SSH client
    let channel_result = tunnel
        .handle
        .channel_open_forwarded_tcpip(
            "localhost",          // connected_address (where the server "received" the connection)
            tunnel.server_port,   // connected_port (the port that was "forwarded")
            "127.0.0.1",          // originator_address
            12345,                // originator_port (arbitrary)
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

    // Build HTTP request to send through the tunnel
    let method = req.method().clone();
    let uri = req.uri().clone();
    let path = uri.path_and_query().map(|p| p.to_string()).unwrap_or_else(|| "/".to_string());
    
    // Construct HTTP/1.1 request
    let http_request = format!(
        "{} {} HTTP/1.1\r\nHost: localhost:{}\r\nConnection: close\r\n\r\n",
        method, path, tunnel.requested_port
    );

    // Send the HTTP request through the channel
    if let Err(e) = channel.data(http_request.as_bytes()).await {
        error!("Failed to send data through channel: {:?}", e);
        return Ok(Response::builder()
            .status(StatusCode::BAD_GATEWAY)
            .body(Full::new(Bytes::from("Failed to send request through tunnel")))
            .unwrap());
    }

    // Send EOF to indicate we're done sending
    channel.eof().await.ok();

    // Read response from the channel
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

    // Parse the HTTP response (simple parsing)
    let response_str = String::from_utf8_lossy(&response_data);
    
    // Find the body (after \r\n\r\n)
    if let Some(body_start) = response_str.find("\r\n\r\n") {
        let body = &response_data[body_start + 4..];
        
        // Try to extract status code
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
        // No proper HTTP response, return raw data
        Ok(Response::builder()
            .status(StatusCode::OK)
            .body(Full::new(Bytes::from(response_data)))
            .unwrap())
    }
}

/// Run the HTTP proxy server
async fn run_http_proxy(state: Arc<AppState>, addr: &str) -> anyhow::Result<()> {
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

// =============================================================================
// Server Key Generation
// =============================================================================

fn generate_server_key() -> anyhow::Result<russh_keys::PrivateKey> {
    use russh_keys::Algorithm;
    
    info!("Generating new Ed25519 server key...");
    let key = russh_keys::PrivateKey::random(&mut rand::thread_rng(), Algorithm::Ed25519)?;
    
    info!("Server key fingerprint: {}", key.public_key().fingerprint(HashAlg::Sha256));
    Ok(key)
}

// =============================================================================
// Main Entry Point
// =============================================================================

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info")
    ).init();

    info!("🚀 Starting SSH Reverse Tunnel Server with HTTP Proxy...");

    // Create shared state
    let state = Arc::new(AppState::new());
    info!("✓ Application state initialized");

    // Generate server key
    let key = generate_server_key()?;

    // Configure SSH server
    let config = russh::server::Config {
        methods: russh::MethodSet::PUBLICKEY,
        server_id: russh::SshId::Standard("SSH-2.0-tunnl-0.1.0".to_string()),
        keys: vec![key],
        inactivity_timeout: Some(std::time::Duration::from_secs(1800)),
        auth_rejection_time: std::time::Duration::from_secs(3),
        auth_rejection_time_initial: Some(std::time::Duration::from_secs(0)),
        ..Default::default()
    };

    let config = Arc::new(config);
    let mut server = TunnelServer::new(state.clone());

    // SSH server address
    let ssh_addr = "0.0.0.0:2222";
    // HTTP proxy address
    let http_addr = "0.0.0.0:8080";

    info!("═══════════════════════════════════════════════════════════════");
    info!("SSH server:   {}", ssh_addr);
    info!("HTTP proxy:   {}", http_addr);
    info!("═══════════════════════════════════════════════════════════════");
    info!("To create a tunnel:");
    info!("  ssh -o StrictHostKeyChecking=no -R 80:localhost:3000 -p 2222 user@localhost");
    info!("");
    info!("Then access via HTTP:");
    info!("  curl -H \"Host: tunnel-xxx.localhost\" http://localhost:8080/");
    info!("═══════════════════════════════════════════════════════════════");

    // Run both servers concurrently
    let http_state = state.clone();
    
    tokio::select! {
        result = server.run_on_address(config, ssh_addr) => {
            result?;
        }
        result = run_http_proxy(http_state, http_addr) => {
            result?;
        }
    }

    Ok(())
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tunnel_error_display() {
        let err = TunnelError::AuthFailed("invalid key".to_string());
        assert_eq!(format!("{}", err), "Authentication failed: invalid key");

        let err = TunnelError::SubdomainTaken("test-123".to_string());
        assert_eq!(format!("{}", err), "Subdomain 'test-123' is already taken");

        let err = TunnelError::TunnelNotFound("unknown".to_string());
        assert_eq!(format!("{}", err), "Tunnel not found for subdomain 'unknown'");
    }

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

    #[tokio::test]
    async fn test_app_state_register_tunnel() {
        // Note: We can't easily test TunnelInfo with Handle in unit tests
        // because Handle requires an actual SSH session.
        // This test is simplified.
        let state = AppState::new();
        assert!(state.list_tunnels().await.is_empty());
    }

    #[tokio::test]
    async fn test_app_state_get_nonexistent() {
        let state = AppState::new();
        assert!(state.get_tunnel("nonexistent").await.is_none());
    }

    #[test]
    fn test_ssh_handler_generate_subdomain() {
        let state = Arc::new(AppState::new());
        let mut handler = SshHandler::new(state, None);
        
        let sub1 = handler.generate_subdomain();
        let sub2 = handler.generate_subdomain();
        
        assert_ne!(sub1, sub2);
        assert!(sub1.starts_with("tunnel-"));
        assert!(sub2.starts_with("tunnel-"));
    }
}
