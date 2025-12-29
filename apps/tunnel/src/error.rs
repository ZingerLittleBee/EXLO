//! Error types for the tunnel server.

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
