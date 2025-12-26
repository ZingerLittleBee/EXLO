//! State management for tunnel registry.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use log::info;
use russh::server::Handle;
use tokio::sync::RwLock;

use crate::error::TunnelError;

/// How long a verified key remains valid (30 minutes)
const VERIFIED_KEY_TTL: Duration = Duration::from_secs(30 * 60);

/// How long a disconnected tunnel remains in the list (same as verified key TTL)
const DISCONNECTED_TUNNEL_TTL: Duration = Duration::from_secs(30 * 60);

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
    /// Whether the SSH connection is still active
    pub is_connected: bool,
    /// When the tunnel was disconnected (None if still connected)
    pub disconnected_at: Option<Instant>,
}

/// A verified public key with expiration
#[derive(Debug, Clone)]
pub struct VerifiedKey {
    pub user_id: String,
    pub verified_at: Instant,
    /// Last used subdomain for this key (to preserve on reconnect)
    pub last_subdomain: Option<String>,
}

impl VerifiedKey {
    pub fn new(user_id: String) -> Self {
        Self {
            user_id,
            verified_at: Instant::now(),
            last_subdomain: None,
        }
    }

    pub fn is_expired(&self) -> bool {
        self.verified_at.elapsed() > VERIFIED_KEY_TTL
    }
}

/// Thread-safe global state for the tunnel registry.
#[derive(Debug, Default)]
pub struct AppState {
    /// Map from subdomain -> TunnelInfo
    pub tunnels: RwLock<HashMap<String, TunnelInfo>>,
    /// Map from public key fingerprint -> VerifiedKey
    pub verified_keys: RwLock<HashMap<String, VerifiedKey>>,
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

    /// Save a verified public key fingerprint
    pub async fn save_verified_key(&self, fingerprint: &str, user_id: &str, subdomain: Option<&str>) {
        let mut keys = self.verified_keys.write().await;
        info!(
            "Saving verified key: fingerprint={}, user_id={}, subdomain={:?}",
            fingerprint, user_id, subdomain
        );
        let mut key = VerifiedKey::new(user_id.to_string());
        key.last_subdomain = subdomain.map(|s| s.to_string());
        keys.insert(fingerprint.to_string(), key);
    }

    /// Update the subdomain for a verified key
    pub async fn update_verified_key_subdomain(&self, fingerprint: &str, subdomain: &str) {
        let mut keys = self.verified_keys.write().await;
        if let Some(key) = keys.get_mut(fingerprint) {
            key.last_subdomain = Some(subdomain.to_string());
            info!("Updated verified key subdomain: fingerprint={}, subdomain={}", fingerprint, subdomain);
        }
    }

    /// Get a verified key if it exists and is not expired
    pub async fn get_verified_key(&self, fingerprint: &str) -> Option<VerifiedKey> {
        let keys = self.verified_keys.read().await;
        keys.get(fingerprint).and_then(|key| {
            if key.is_expired() {
                None
            } else {
                Some(key.clone())
            }
        })
    }

    /// Clean up expired verified keys
    pub async fn cleanup_expired_keys(&self) {
        let mut keys = self.verified_keys.write().await;
        keys.retain(|_, key| !key.is_expired());
    }

    /// Mark a tunnel as disconnected (but keep it for reconnection window)
    pub async fn mark_tunnel_disconnected(&self, subdomain: &str) {
        let mut tunnels = self.tunnels.write().await;
        if let Some(tunnel) = tunnels.get_mut(subdomain) {
            tunnel.is_connected = false;
            tunnel.disconnected_at = Some(Instant::now());
            info!("Marked tunnel as disconnected: {}", subdomain);
        }
    }

    /// Clean up tunnels that have been disconnected for too long
    pub async fn cleanup_expired_tunnels(&self) {
        let mut tunnels = self.tunnels.write().await;
        tunnels.retain(|subdomain, tunnel| {
            if let Some(disconnected_at) = tunnel.disconnected_at {
                if disconnected_at.elapsed() > DISCONNECTED_TUNNEL_TTL {
                    info!("Removing expired disconnected tunnel: {}", subdomain);
                    return false;
                }
            }
            true
        });
    }
}
