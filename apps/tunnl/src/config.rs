//! Centralized configuration management for the tunnel server.
//!
//! All configuration must be provided via environment variables.
//! Missing required variables will cause a panic at startup.

use std::sync::OnceLock;

// ============================================================================
// Environment variable names
// ============================================================================

mod env {
    pub const TUNNEL_URL: &str = "TUNNEL_URL";
    pub const API_BASE_URL: &str = "API_BASE_URL";
    pub const INTERNAL_API_SECRET: &str = "INTERNAL_API_SECRET";
}

/// Minimum length for INTERNAL_API_SECRET
const MIN_SECRET_LENGTH: usize = 32;

// ============================================================================
// Global configuration (loaded once at startup)
// ============================================================================

static CONFIG: OnceLock<Config> = OnceLock::new();

#[derive(Debug, Clone)]
pub struct Config {
    /// Base domain for tunnels (e.g., "tunnel.example.com" or "localhost:8080")
    pub tunnel_url: String,
    pub api_base_url: String,
    pub internal_api_secret: String,
}

impl Config {
    fn load() -> Self {
        let tunnel_url = std::env::var(env::TUNNEL_URL)
            .unwrap_or_else(|_| panic!("{} environment variable is required", env::TUNNEL_URL));

        let api_base_url = std::env::var(env::API_BASE_URL)
            .unwrap_or_else(|_| panic!("{} environment variable is required", env::API_BASE_URL));

        let internal_api_secret = std::env::var(env::INTERNAL_API_SECRET).unwrap_or_else(|_| {
            panic!(
                "{} environment variable is required",
                env::INTERNAL_API_SECRET
            )
        });

        let config = Self {
            tunnel_url,
            api_base_url,
            internal_api_secret,
        };

        config.validate();
        config
    }

    fn validate(&self) {
        if self.internal_api_secret.len() < MIN_SECRET_LENGTH {
            panic!(
                "{} must be at least {} characters",
                env::INTERNAL_API_SECRET, MIN_SECRET_LENGTH
            );
        }
    }
}

// ============================================================================
// Public API
// ============================================================================

/// Initialize configuration. Must be called once at startup.
/// Panics if required environment variables are missing.
pub fn init() {
    CONFIG.get_or_init(Config::load);
}

/// Get the global configuration. Panics if not initialized.
pub fn get() -> &'static Config {
    CONFIG.get().expect("Config not initialized. Call config::init() first.")
}

/// Construct a tunnel address from subdomain (without protocol)
pub fn get_tunnel_url(subdomain: &str) -> String {
    let config = get();
    format!("{}.{}", subdomain, config.tunnel_url)
}
