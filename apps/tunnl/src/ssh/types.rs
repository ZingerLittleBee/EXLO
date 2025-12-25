//! SSH handler types and shared state definitions.

use russh::server::Handle;
use russh::ChannelId;

/// A pending tunnel request waiting for verification
#[derive(Debug, Clone)]
pub struct PendingTunnel {
    pub address: String,
    pub port: u32,
}

/// Shared state that can be accessed from the polling task
pub struct SharedHandlerState {
    pub verification_status: VerificationStatus,
    pub pending_tunnels: Vec<PendingTunnel>,
    pub registered_subdomains: Vec<String>,
    pub subdomain_counter: u32,
    /// Session handle for sending data to client (set after auth succeeds)
    pub session_handle: Option<Handle>,
    /// Session channel ID (set when session channel is opened)
    pub session_channel_id: Option<ChannelId>,
    /// Whether ESC was pressed once (for double-ESC to disconnect)
    pub esc_pressed: bool,
    /// Timestamp of last ESC press for timeout
    pub last_esc_time: Option<std::time::Instant>,
    /// Last subdomain from previous session (for reconnection)
    pub last_subdomain: Option<String>,
    /// Port for the reconnect message (set when tunnel created before session channel opens)
    pub pending_reconnect_port: Option<u32>,
}

impl SharedHandlerState {
    pub fn new() -> Self {
        Self {
            verification_status: VerificationStatus::NotStarted,
            pending_tunnels: Vec::new(),
            registered_subdomains: Vec::new(),
            subdomain_counter: 0,
            session_handle: None,
            session_channel_id: None,
            esc_pressed: false,
            last_esc_time: None,
            last_subdomain: None,
            pending_reconnect_port: None,
        }
    }
}

impl Default for SharedHandlerState {
    fn default() -> Self {
        Self::new()
    }
}

/// Device Flow verification status
#[derive(Debug, Clone, PartialEq)]
pub enum VerificationStatus {
    /// Not yet started
    NotStarted,
    /// Waiting for user to verify via web
    Pending { code: String },
    /// Verified with user ID
    Verified { user_id: String },
    /// Verification failed or timed out
    Failed { reason: String },
}

pub fn generate_session_id() -> String {
    use std::time::SystemTime;
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("ssh-{:x}", now)
}

pub fn rand_simple() -> u32 {
    use std::time::SystemTime;
    let duration = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    ((duration.as_nanos() as u64 ^ 0x5DEECE66D) & 0xFFFFFF) as u32
}
