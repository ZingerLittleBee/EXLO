//! SSH Reverse Tunnel Server
//!
//! A high-performance SSH server that handles reverse port forwarding requests
//! using "virtual bind" - we don't actually bind TCP ports, just register tunnels
//! in a shared map for later routing by an HTTP proxy layer.
//!
//! ## Usage
//! ```bash
//! ssh -o StrictHostKeyChecking=no -R 80:localhost:3000 -p 2222 test@localhost
//! ```

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use log::{debug, error, info, warn};
use russh::keys::PublicKey;
use russh::server::{Auth, Handler, Msg, Server, Session};
use russh::{Channel, ChannelId};
use russh_keys::HashAlg;
use tokio::sync::RwLock;

// =============================================================================
// Error Types (using thiserror for domain-specific errors)
// =============================================================================

/// Custom error types for tunnel-related operations.
/// These are domain-specific errors that can be handled gracefully.
#[derive(Debug, thiserror::Error)]
pub enum TunnelError {
    /// Authentication failed for the given reason
    #[error("Authentication failed: {0}")]
    AuthFailed(String),

    /// The requested subdomain is already registered
    #[error("Subdomain '{0}' is already taken")]
    SubdomainTaken(String),

    /// Tunnel not found when trying to remove or access
    #[error("Tunnel not found for subdomain '{0}'")]
    TunnelNotFound(String),

    /// Underlying SSH protocol error
    #[error("SSH protocol error: {0}")]
    SshError(#[from] russh::Error),

    /// I/O error
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
}

// =============================================================================
// State Types
// =============================================================================

/// Information about a registered tunnel.
/// This is stored in the global AppState and used by the HTTP proxy layer
/// to route incoming requests to the correct SSH channel.
#[derive(Debug, Clone)]
pub struct TunnelInfo {
    /// The assigned subdomain (e.g., "abc123")
    pub subdomain: String,
    /// The address the client requested to forward (from `-R address:port:...`)
    pub requested_address: String,
    /// The port the client requested (from `-R ...:port:...`)
    pub requested_port: u32,
    /// When this tunnel was created
    pub created_at: Instant,
    /// The client's username
    pub username: String,
}

/// Thread-safe global state for the tunnel registry.
/// This is shared across all SSH connections and used by:
/// 1. SSH handlers to register/unregister tunnels
/// 2. HTTP proxy layer (not implemented here) to route requests
#[derive(Debug, Default)]
pub struct AppState {
    /// Map from subdomain -> TunnelInfo
    /// Using RwLock for concurrent read access, exclusive write access
    pub tunnels: RwLock<HashMap<String, TunnelInfo>>,
}

impl AppState {
    /// Create a new empty AppState
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a new tunnel. Returns error if subdomain is already taken.
    pub async fn register_tunnel(&self, info: TunnelInfo) -> Result<(), TunnelError> {
        let mut tunnels = self.tunnels.write().await;
        if tunnels.contains_key(&info.subdomain) {
            return Err(TunnelError::SubdomainTaken(info.subdomain));
        }
        info!("Registered tunnel: {}", info.subdomain);
        tunnels.insert(info.subdomain.clone(), info);
        Ok(())
    }

    /// Remove a tunnel by subdomain
    pub async fn remove_tunnel(&self, subdomain: &str) -> Result<TunnelInfo, TunnelError> {
        let mut tunnels = self.tunnels.write().await;
        tunnels
            .remove(subdomain)
            .ok_or_else(|| TunnelError::TunnelNotFound(subdomain.to_string()))
    }

    /// Get a tunnel by subdomain (read-only)
    pub async fn get_tunnel(&self, subdomain: &str) -> Option<TunnelInfo> {
        let tunnels = self.tunnels.read().await;
        tunnels.get(subdomain).cloned()
    }

    /// List all active tunnels
    pub async fn list_tunnels(&self) -> Vec<TunnelInfo> {
        let tunnels = self.tunnels.read().await;
        tunnels.values().cloned().collect()
    }
}

// =============================================================================
// SSH Server Implementation
// =============================================================================

/// The main SSH server that creates handlers for each connection.
/// Implements `russh::server::Server` trait.
#[derive(Clone)]
pub struct TunnelServer {
    /// Shared application state
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

/// Handler for a single SSH connection.
/// Implements `russh::server::Handler` trait with all the callback methods.
pub struct SshHandler {
    /// Shared application state
    state: Arc<AppState>,
    /// Remote peer address
    peer_addr: Option<SocketAddr>,
    /// Username after authentication
    username: Option<String>,
    /// Subdomains registered by this connection (for cleanup on disconnect)
    registered_subdomains: Vec<String>,
    /// Counter for generating unique subdomains per connection
    subdomain_counter: u32,
}

impl SshHandler {
    pub fn new(state: Arc<AppState>, peer_addr: Option<SocketAddr>) -> Self {
        Self {
            state,
            peer_addr,
            username: None,
            registered_subdomains: Vec::new(),
            subdomain_counter: 0,
        }
    }

    /// Generate a unique subdomain for this tunnel.
    /// In production, this might use user preferences or random strings.
    fn generate_subdomain(&mut self) -> String {
        self.subdomain_counter += 1;
        let random_part: u32 = rand_simple();
        format!("tunnel-{:06x}-{}", random_part, self.subdomain_counter)
    }
}

/// Simple pseudo-random number generator (no external dependency)
fn rand_simple() -> u32 {
    use std::time::SystemTime;
    let duration = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    // Mix nanoseconds for some randomness
    ((duration.as_nanos() as u64 ^ 0x5DEECE66D) & 0xFFFFFF) as u32
}

#[async_trait]
impl Handler for SshHandler {
    type Error = TunnelError;

    /// Called when the client disconnects or the session ends.
    /// We use this to clean up any registered tunnels.
    async fn channel_close(
        &mut self,
        channel: ChannelId,
        _session: &mut Session,
    ) -> Result<(), Self::Error> {
        info!("Channel {:?} closed, cleaning up tunnels...", channel);
        
        // Remove all tunnels registered by this connection
        for subdomain in &self.registered_subdomains {
            match self.state.remove_tunnel(subdomain).await {
                Ok(_) => info!("Removed tunnel: {}", subdomain),
                Err(e) => warn!("Failed to remove tunnel {}: {}", subdomain, e),
            }
        }
        self.registered_subdomains.clear();
        
        Ok(())
    }

    /// Public key authentication.
    /// For now, we accept all keys but log the fingerprint.
    /// In production, you'd validate against a user database.
    async fn auth_publickey(
        &mut self,
        user: &str,
        public_key: &PublicKey,
    ) -> Result<Auth, Self::Error> {
        // Get the key fingerprint for logging (SHA-256)
        let fingerprint = public_key.fingerprint(HashAlg::Sha256);
        
        info!(
            "Public key auth attempt: user='{}', fingerprint='{}'",
            user, fingerprint
        );
        
        // Store the username for later use
        self.username = Some(user.to_string());
        
        // Accept all keys for now (development mode)
        // TODO: In production, validate against authorized_keys or a user database
        Ok(Auth::Accept)
    }

    /// Handle the `-R` (reverse port forwarding) request from the client.
    /// 
    /// ## CRITICAL: Virtual Bind Logic
    /// 
    /// When the client runs `ssh -R 80:localhost:3000 server`, this method is called
    /// with `address=""` and `port=80`. Traditional SSH servers would bind port 80
    /// on the server. **We do NOT do this.**
    /// 
    /// Instead, we:
    /// 1. Generate a unique subdomain (e.g., "tunnel-abc123-1")
    /// 2. Store the mapping in our AppState
    /// 3. Return `true` to tell the client "forwarding succeeded"
    /// 
    /// The actual port 80 is handled by a separate HTTP proxy layer (not in this file)
    /// that looks up the subdomain and routes traffic to the SSH channel.
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

        // =====================================================================
        // VIRTUAL BIND: We do NOT actually bind a TCP port here!
        // Instead, we only REGISTER the tunnel in our shared state.
        // This is the key innovation that allows services like ngrok/tunnl.gg
        // to scale to thousands of concurrent tunnels without port exhaustion.
        // =====================================================================

        let subdomain = self.generate_subdomain();
        let username = self.username.clone().unwrap_or_else(|| "anonymous".to_string());

        let tunnel_info = TunnelInfo {
            subdomain: subdomain.clone(),
            requested_address: address.to_string(),
            requested_port: *port,
            created_at: Instant::now(),
            username,
        };

        // Register in global state
        match self.state.register_tunnel(tunnel_info).await {
            Ok(()) => {
                info!(
                    "✓ Virtual tunnel registered!\n\
                     Subdomain: {}\n\
                     URL: https://{}.example.com",
                    subdomain, subdomain
                );
                self.registered_subdomains.push(subdomain);
                
                // Return true to tell the SSH client the forward was successful
                // (even though we didn't actually bind a port!)
                Ok(true)
            }
            Err(TunnelError::SubdomainTaken(s)) => {
                warn!("Subdomain {} already taken, rejecting forward request", s);
                Ok(false)
            }
            Err(e) => {
                error!("Failed to register tunnel: {}", e);
                Err(e)
            }
        }
    }

    /// Handle cancellation of a reverse port forward.
    async fn cancel_tcpip_forward(
        &mut self,
        address: &str,
        port: u32,
        _session: &mut Session,
    ) -> Result<bool, Self::Error> {
        info!(
            "Cancel tcpip_forward request: address='{}', port={}",
            address, port
        );

        // Find and remove tunnels matching this address/port
        // (In practice, we'd track the mapping more precisely)
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

    /// Handle a session channel open request.
    /// We accept sessions but don't provide an interactive shell.
    async fn channel_open_session(
        &mut self,
        channel: Channel<Msg>,
        _session: &mut Session,
    ) -> Result<bool, Self::Error> {
        info!(
            "Session channel opened: id={:?}, user={:?}",
            channel.id(),
            self.username
        );
        
        // Accept the session channel
        // The client can use this for keepalive or other control messages
        Ok(true)
    }

    /// Handle incoming data on a channel.
    /// For a tunnel server, this is mostly boilerplate - the real data
    /// flow happens through the forwarded TCP connections.
    async fn data(
        &mut self,
        channel: ChannelId,
        data: &[u8],
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        debug!(
            "Data received on channel {:?}: {} bytes",
            channel,
            data.len()
        );

        // Echo the data back (for testing purposes)
        // In production, you might handle control commands here
        session.data(channel, data.to_vec().into())?;
        
        Ok(())
    }

    /// Handle EOF on a channel
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
// Server Key Generation
// =============================================================================

/// Generate or load an SSH server key.
/// In production, you'd load this from a file or HSM.
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
    // Initialize logging
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info")
    ).init();

    info!("🚀 Starting SSH Reverse Tunnel Server...");

    // Create shared state
    let state = Arc::new(AppState::new());
    info!("✓ Application state initialized");

    // Generate server key
    let key = generate_server_key()?;

    // Configure the SSH server
    let config = russh::server::Config {
        // Authentication methods - we only support pubkey for now
        methods: russh::MethodSet::PUBLICKEY,
        
        // Server identification string
        server_id: russh::SshId::Standard(
            "SSH-2.0-tunnl-0.1.0".to_string()
        ),
        
        // Keys for the server
        keys: vec![key],
        
        // Timeouts and limits
        inactivity_timeout: Some(std::time::Duration::from_secs(1800)), // 30 minutes
        auth_rejection_time: std::time::Duration::from_secs(3),
        auth_rejection_time_initial: Some(std::time::Duration::from_secs(0)),
        
        ..Default::default()
    };

    let config = Arc::new(config);

    // Create the server
    let mut server = TunnelServer::new(state.clone());

    // Bind address
    let addr = "0.0.0.0:2222";
    info!("✓ Listening on {}", addr);
    info!("═══════════════════════════════════════════════════════════════");
    info!("To test, run:");
    info!("  ssh -o StrictHostKeyChecking=no -R 80:localhost:3000 -p 2222 test@localhost");
    info!("═══════════════════════════════════════════════════════════════");

    // Run the server using the Server trait method
    server.run_on_address(config, addr).await?;

    Ok(())
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // TunnelError Tests
    // -------------------------------------------------------------------------

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
    fn test_tunnel_error_debug() {
        let err = TunnelError::AuthFailed("test".to_string());
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("AuthFailed"));
    }

    // -------------------------------------------------------------------------
    // AppState Tests
    // -------------------------------------------------------------------------

    #[tokio::test]
    async fn test_app_state_register_tunnel() {
        let state = AppState::new();
        
        let info = TunnelInfo {
            subdomain: "test-subdomain".to_string(),
            requested_address: "localhost".to_string(),
            requested_port: 80,
            created_at: Instant::now(),
            username: "testuser".to_string(),
        };
        
        // Should succeed first time
        assert!(state.register_tunnel(info.clone()).await.is_ok());
        
        // Should fail second time (duplicate)
        let result = state.register_tunnel(info).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TunnelError::SubdomainTaken(_)));
    }

    #[tokio::test]
    async fn test_app_state_remove_tunnel() {
        let state = AppState::new();
        
        let info = TunnelInfo {
            subdomain: "remove-me".to_string(),
            requested_address: "0.0.0.0".to_string(),
            requested_port: 8080,
            created_at: Instant::now(),
            username: "user".to_string(),
        };
        
        state.register_tunnel(info).await.unwrap();
        
        // Should succeed
        let removed = state.remove_tunnel("remove-me").await;
        assert!(removed.is_ok());
        assert_eq!(removed.unwrap().subdomain, "remove-me");
        
        // Should fail (already removed)
        let result = state.remove_tunnel("remove-me").await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TunnelError::TunnelNotFound(_)));
    }

    #[tokio::test]
    async fn test_app_state_get_tunnel() {
        let state = AppState::new();
        
        // Should return None for non-existent
        assert!(state.get_tunnel("nope").await.is_none());
        
        let info = TunnelInfo {
            subdomain: "findme".to_string(),
            requested_address: "".to_string(),
            requested_port: 443,
            created_at: Instant::now(),
            username: "finder".to_string(),
        };
        
        state.register_tunnel(info).await.unwrap();
        
        // Should find it now
        let found = state.get_tunnel("findme").await;
        assert!(found.is_some());
        assert_eq!(found.unwrap().requested_port, 443);
    }

    #[tokio::test]
    async fn test_app_state_list_tunnels() {
        let state = AppState::new();
        
        // Empty initially
        assert!(state.list_tunnels().await.is_empty());
        
        // Add some tunnels
        for i in 0..3 {
            let info = TunnelInfo {
                subdomain: format!("tunnel-{}", i),
                requested_address: "".to_string(),
                requested_port: 80 + i,
                created_at: Instant::now(),
                username: "user".to_string(),
            };
            state.register_tunnel(info).await.unwrap();
        }
        
        let tunnels = state.list_tunnels().await;
        assert_eq!(tunnels.len(), 3);
    }

    #[tokio::test]
    async fn test_app_state_concurrent_access() {
        use std::sync::Arc;
        
        let state = Arc::new(AppState::new());
        let mut handles = vec![];
        
        // Spawn multiple tasks that register tunnels concurrently
        for i in 0..10 {
            let state = state.clone();
            handles.push(tokio::spawn(async move {
                let info = TunnelInfo {
                    subdomain: format!("concurrent-{}", i),
                    requested_address: "".to_string(),
                    requested_port: 8000 + i,
                    created_at: Instant::now(),
                    username: format!("user-{}", i),
                };
                state.register_tunnel(info).await
            }));
        }
        
        // All should succeed (unique subdomains)
        for handle in handles {
            assert!(handle.await.unwrap().is_ok());
        }
        
        assert_eq!(state.list_tunnels().await.len(), 10);
    }

    // -------------------------------------------------------------------------
    // SshHandler Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_ssh_handler_generate_subdomain() {
        let state = Arc::new(AppState::new());
        let mut handler = SshHandler::new(state, None);
        
        let sub1 = handler.generate_subdomain();
        let sub2 = handler.generate_subdomain();
        
        // Should be different
        assert_ne!(sub1, sub2);
        
        // Should have expected format
        assert!(sub1.starts_with("tunnel-"));
        assert!(sub2.starts_with("tunnel-"));
    }
}
