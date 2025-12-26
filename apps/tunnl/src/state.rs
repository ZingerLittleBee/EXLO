//! State management for tunnel registry.

use std::collections::HashMap;
use std::net::IpAddr;
use std::time::{Duration, SystemTime};

use log::info;
use russh::server::Handle;
use tokio::sync::RwLock;

use crate::error::TunnelError;

/// How long a verified key remains valid (30 minutes)
const VERIFIED_KEY_TTL: Duration = Duration::from_secs(30 * 60);

/// How long a disconnected tunnel remains in the list (same as verified key TTL)
const DISCONNECTED_TUNNEL_TTL: Duration = Duration::from_secs(30 * 60);

/// Minimum interval between Device Flow requests per IP (10 seconds)
const DEVICE_FLOW_RATE_LIMIT: Duration = Duration::from_secs(10);

/// Maximum Device Flow attempts per IP within the rate limit window (5 attempts per minute)
const DEVICE_FLOW_MAX_ATTEMPTS: u32 = 5;

/// Window for counting Device Flow attempts (1 minute)
const DEVICE_FLOW_WINDOW: Duration = Duration::from_secs(60);

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
    /// When this tunnel was created (wall-clock time for persistence)
    pub created_at: SystemTime,
    /// The client's username
    pub username: String,
    /// The client's IP address
    pub client_ip: String,
    /// Whether the SSH connection is still active
    pub is_connected: bool,
    /// When the tunnel was disconnected (None if still connected)
    pub disconnected_at: Option<SystemTime>,
}

/// A verified public key with expiration
#[derive(Debug, Clone)]
pub struct VerifiedKey {
    pub user_id: String,
    pub verified_at: SystemTime,
    /// Last used subdomain for this key (to preserve on reconnect)
    pub last_subdomain: Option<String>,
}

impl VerifiedKey {
    pub fn new(user_id: String) -> Self {
        Self {
            user_id,
            verified_at: SystemTime::now(),
            last_subdomain: None,
        }
    }

    pub fn is_expired(&self) -> bool {
        SystemTime::now()
            .duration_since(self.verified_at)
            .map(|elapsed| elapsed > VERIFIED_KEY_TTL)
            .unwrap_or(true)
    }
}

/// Rate limit tracking for Device Flow requests
#[derive(Debug, Clone)]
pub struct RateLimitEntry {
    pub last_request: SystemTime,
    pub attempts: u32,
    pub window_start: SystemTime,
}

impl RateLimitEntry {
    pub fn new() -> Self {
        let now = SystemTime::now();
        Self {
            last_request: now,
            attempts: 1,
            window_start: now,
        }
    }

    pub fn is_rate_limited(&self) -> bool {
        let now = SystemTime::now();
        
        // Check minimum interval since last request
        if let Ok(since_last) = now.duration_since(self.last_request) {
            if since_last < DEVICE_FLOW_RATE_LIMIT {
                return true;
            }
        }
        
        // Check max attempts in window
        if let Ok(since_window_start) = now.duration_since(self.window_start) {
            if since_window_start < DEVICE_FLOW_WINDOW && self.attempts >= DEVICE_FLOW_MAX_ATTEMPTS {
                return true;
            }
        }
        
        false
    }

    pub fn record_attempt(&mut self) {
        let now = SystemTime::now();
        
        // Reset window if expired
        if let Ok(since_window_start) = now.duration_since(self.window_start) {
            if since_window_start >= DEVICE_FLOW_WINDOW {
                self.attempts = 0;
                self.window_start = now;
            }
        }
        
        self.last_request = now;
        self.attempts += 1;
    }
}

impl Default for RateLimitEntry {
    fn default() -> Self {
        Self::new()
    }
}

/// Thread-safe global state for the tunnel registry.
#[derive(Debug, Default)]
pub struct AppState {
    /// Map from subdomain -> TunnelInfo
    pub tunnels: RwLock<HashMap<String, TunnelInfo>>,
    /// Map from public key fingerprint -> VerifiedKey
    pub verified_keys: RwLock<HashMap<String, VerifiedKey>>,
    /// Rate limiting for Device Flow requests (IP -> RateLimitEntry)
    rate_limits: RwLock<HashMap<IpAddr, RateLimitEntry>>,
}

impl AppState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if an IP is rate-limited for Device Flow requests
    pub async fn is_device_flow_rate_limited(&self, ip: IpAddr) -> bool {
        let limits = self.rate_limits.read().await;
        if let Some(entry) = limits.get(&ip) {
            entry.is_rate_limited()
        } else {
            false
        }
    }

    /// Record a Device Flow request from an IP
    pub async fn record_device_flow_request(&self, ip: IpAddr) {
        let mut limits = self.rate_limits.write().await;
        if let Some(entry) = limits.get_mut(&ip) {
            entry.record_attempt();
        } else {
            limits.insert(ip, RateLimitEntry::new());
        }
    }

    /// Clean up old rate limit entries
    pub async fn cleanup_rate_limits(&self) {
        let mut limits = self.rate_limits.write().await;
        let now = SystemTime::now();
        limits.retain(|_, entry| {
            now.duration_since(entry.window_start)
                .map(|elapsed| elapsed < DEVICE_FLOW_WINDOW * 2)
                .unwrap_or(false)
        });
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
            tunnel.disconnected_at = Some(SystemTime::now());
            info!("Marked tunnel as disconnected: {}", subdomain);
        }
    }

    /// Clean up tunnels that have been disconnected for too long
    pub async fn cleanup_expired_tunnels(&self) {
        let mut tunnels = self.tunnels.write().await;
        let now = SystemTime::now();
        tunnels.retain(|subdomain, tunnel| {
            if let Some(disconnected_at) = tunnel.disconnected_at {
                if let Ok(elapsed) = now.duration_since(disconnected_at) {
                    if elapsed > DISCONNECTED_TUNNEL_TTL {
                        info!("Removing expired disconnected tunnel: {}", subdomain);
                        return false;
                    }
                }
            }
            true
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    fn create_test_state() -> AppState {
        AppState::new()
    }

    #[test]
    fn test_verified_key_expiration() {
        let key = VerifiedKey::new("user123".to_string());
        assert!(!key.is_expired());
    }

    #[test]
    fn test_rate_limit_entry_new() {
        let entry = RateLimitEntry::new();
        assert_eq!(entry.attempts, 1);
    }

    #[test]
    fn test_rate_limit_entry_is_rate_limited_on_first_request() {
        let entry = RateLimitEntry::new();
        // Should be rate limited because last_request is just now (< 10s ago)
        assert!(entry.is_rate_limited());
    }

    #[test]
    fn test_rate_limit_entry_max_attempts() {
        let mut entry = RateLimitEntry::new();
        // Record more attempts to exceed limit
        for _ in 0..DEVICE_FLOW_MAX_ATTEMPTS {
            entry.record_attempt();
        }
        assert!(entry.is_rate_limited());
    }

    #[tokio::test]
    async fn test_device_flow_rate_limiting() {
        let state = create_test_state();
        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));

        // First request should not be rate limited
        assert!(!state.is_device_flow_rate_limited(ip).await);

        // Record the request
        state.record_device_flow_request(ip).await;

        // Now should be rate limited (too soon)
        assert!(state.is_device_flow_rate_limited(ip).await);
    }

    #[tokio::test]
    async fn test_verified_key_save_and_get() {
        let state = create_test_state();
        let fingerprint = "SHA256:abc123";
        let user_id = "user1";

        state.save_verified_key(fingerprint, user_id, Some("test-subdomain")).await;

        let key = state.get_verified_key(fingerprint).await;
        assert!(key.is_some());
        let key = key.unwrap();
        assert_eq!(key.user_id, user_id);
        assert_eq!(key.last_subdomain, Some("test-subdomain".to_string()));
    }

    #[tokio::test]
    async fn test_verified_key_not_found() {
        let state = create_test_state();
        let key = state.get_verified_key("nonexistent").await;
        assert!(key.is_none());
    }

    #[tokio::test]
    async fn test_update_verified_key_subdomain() {
        let state = create_test_state();
        let fingerprint = "SHA256:xyz789";

        state.save_verified_key(fingerprint, "user", None).await;
        state.update_verified_key_subdomain(fingerprint, "new-subdomain").await;

        let key = state.get_verified_key(fingerprint).await.unwrap();
        assert_eq!(key.last_subdomain, Some("new-subdomain".to_string()));
    }

    #[tokio::test]
    async fn test_cleanup_rate_limits() {
        let state = create_test_state();
        let ip = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1));

        state.record_device_flow_request(ip).await;
        
        // Should have entry
        {
            let limits = state.rate_limits.read().await;
            assert!(limits.contains_key(&ip));
        }

        // Cleanup should not remove recent entries
        state.cleanup_rate_limits().await;
        
        {
            let limits = state.rate_limits.read().await;
            assert!(limits.contains_key(&ip));
        }
    }
}
