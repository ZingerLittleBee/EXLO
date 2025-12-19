//! State management for tunnel registry.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use log::info;
use russh::server::Handle;
use tokio::sync::RwLock;

use crate::error::TunnelError;

/// Information about a registered tunnel.
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
    /// The client's IP address
    pub client_ip: String,
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
