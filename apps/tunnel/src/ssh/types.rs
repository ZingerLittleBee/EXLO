//! SSH handler types and shared state definitions.

use russh::server::Handle;
use russh::ChannelId;

/// Maximum length for a subdomain (DNS label limit)
pub const MAX_SUBDOMAIN_LENGTH: usize = 63;

/// Minimum length for a subdomain
pub const MIN_SUBDOMAIN_LENGTH: usize = 1;

/// Subdomain validation result
#[derive(Debug, Clone, PartialEq)]
pub enum SubdomainValidation {
    Valid,
    TooLong,
    TooShort,
    InvalidCharacters,
    StartsWithHyphen,
    EndsWithHyphen,
}

/// Validate a subdomain string.
/// 
/// Rules:
/// - Length: 1-63 characters (DNS label limit)
/// - Characters: lowercase letters, digits, hyphens only
/// - Cannot start or end with a hyphen
pub fn validate_subdomain(subdomain: &str) -> SubdomainValidation {
    if subdomain.len() > MAX_SUBDOMAIN_LENGTH {
        return SubdomainValidation::TooLong;
    }
    
    if subdomain.len() < MIN_SUBDOMAIN_LENGTH {
        return SubdomainValidation::TooShort;
    }
    
    if subdomain.starts_with('-') {
        return SubdomainValidation::StartsWithHyphen;
    }
    
    if subdomain.ends_with('-') {
        return SubdomainValidation::EndsWithHyphen;
    }
    
    // Only allow lowercase alphanumeric and hyphens
    if !subdomain.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
        return SubdomainValidation::InvalidCharacters;
    }
    
    SubdomainValidation::Valid
}

/// Check if a subdomain is valid
pub fn is_valid_subdomain(subdomain: &str) -> bool {
    validate_subdomain(subdomain) == SubdomainValidation::Valid
}

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
    /// Subdomains from previous session, keyed by client port (for reconnection)
    /// Maps client_port -> subdomain
    pub last_subdomains: std::collections::HashMap<u32, String>,
    /// Pending tunnel port (set when tunnel created before session channel opens)
    pub pending_tunnel_port: Option<u32>,
    /// User-requested subdomain from SSH username (strict - disconnect on conflict)
    /// None means use random subdomain (when username is ".")
    pub requested_subdomain: Option<String>,
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
            last_subdomains: std::collections::HashMap::new(),
            pending_tunnel_port: None,
            requested_subdomain: None,
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
    /// Verified with user ID and display name
    Verified { user_id: String, display_name: String },
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

/// Generate a cryptographically secure random subdomain string.
/// Uses OsRng for security and produces a 16-character hex string (64 bits of entropy).
pub fn generate_secure_subdomain_id() -> String {
    use rand::RngCore;
    let mut bytes = [0u8; 8];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    hex::encode(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_session_id_format() {
        let session_id = generate_session_id();
        assert!(session_id.starts_with("ssh-"));
        assert!(session_id.len() > 4);
    }

    #[test]
    fn test_generate_session_id_unique() {
        let id1 = generate_session_id();
        std::thread::sleep(std::time::Duration::from_nanos(1));
        let id2 = generate_session_id();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_generate_secure_subdomain_id_length() {
        let subdomain = generate_secure_subdomain_id();
        // 8 bytes = 16 hex characters
        assert_eq!(subdomain.len(), 16);
    }

    #[test]
    fn test_generate_secure_subdomain_id_is_hex() {
        let subdomain = generate_secure_subdomain_id();
        assert!(subdomain.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_generate_secure_subdomain_id_unique() {
        let id1 = generate_secure_subdomain_id();
        let id2 = generate_secure_subdomain_id();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_shared_handler_state_default() {
        let state = SharedHandlerState::new();
        assert!(matches!(state.verification_status, VerificationStatus::NotStarted));
        assert!(state.pending_tunnels.is_empty());
        assert!(state.registered_subdomains.is_empty());
        assert_eq!(state.subdomain_counter, 0);
    }

    #[test]
    fn test_verification_status_equality() {
        let status1 = VerificationStatus::Verified { user_id: "user1".to_string(), display_name: "User 1".to_string() };
        let status2 = VerificationStatus::Verified { user_id: "user1".to_string(), display_name: "User 1".to_string() };
        let status3 = VerificationStatus::Verified { user_id: "user2".to_string(), display_name: "User 2".to_string() };
        
        assert_eq!(status1, status2);
        assert_ne!(status1, status3);
    }

    #[test]
    fn test_pending_tunnel_clone() {
        let tunnel = PendingTunnel {
            address: "localhost".to_string(),
            port: 3000,
        };
        let cloned = tunnel.clone();
        assert_eq!(tunnel.address, cloned.address);
        assert_eq!(tunnel.port, cloned.port);
    }

    // Subdomain validation tests
    #[test]
    fn test_validate_subdomain_valid() {
        assert_eq!(validate_subdomain("myapp"), SubdomainValidation::Valid);
        assert_eq!(validate_subdomain("my-app"), SubdomainValidation::Valid);
        assert_eq!(validate_subdomain("app123"), SubdomainValidation::Valid);
        assert_eq!(validate_subdomain("a"), SubdomainValidation::Valid);
        assert_eq!(validate_subdomain("a1b2c3"), SubdomainValidation::Valid);
    }

    #[test]
    fn test_validate_subdomain_too_long() {
        let long_subdomain = "a".repeat(64);
        assert_eq!(validate_subdomain(&long_subdomain), SubdomainValidation::TooLong);
        
        // 63 chars should be valid
        let max_subdomain = "a".repeat(63);
        assert_eq!(validate_subdomain(&max_subdomain), SubdomainValidation::Valid);
    }

    #[test]
    fn test_validate_subdomain_too_short() {
        assert_eq!(validate_subdomain(""), SubdomainValidation::TooShort);
    }

    #[test]
    fn test_validate_subdomain_invalid_characters() {
        assert_eq!(validate_subdomain("my_app"), SubdomainValidation::InvalidCharacters);
        assert_eq!(validate_subdomain("my.app"), SubdomainValidation::InvalidCharacters);
        assert_eq!(validate_subdomain("my@app"), SubdomainValidation::InvalidCharacters);
        assert_eq!(validate_subdomain("MY-APP"), SubdomainValidation::InvalidCharacters); // uppercase not allowed
        assert_eq!(validate_subdomain("app!"), SubdomainValidation::InvalidCharacters);
    }

    #[test]
    fn test_validate_subdomain_hyphen_position() {
        assert_eq!(validate_subdomain("-myapp"), SubdomainValidation::StartsWithHyphen);
        assert_eq!(validate_subdomain("myapp-"), SubdomainValidation::EndsWithHyphen);
        assert_eq!(validate_subdomain("-"), SubdomainValidation::StartsWithHyphen);
    }

    #[test]
    fn test_is_valid_subdomain() {
        assert!(is_valid_subdomain("myapp"));
        assert!(is_valid_subdomain("my-app-123"));
        assert!(!is_valid_subdomain(""));
        assert!(!is_valid_subdomain("-app"));
        assert!(!is_valid_subdomain("app-"));
        assert!(!is_valid_subdomain("MY_APP"));
    }
}
