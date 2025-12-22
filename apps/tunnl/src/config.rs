//! Centralized configuration management for the tunnel server.
//!
//! This module provides environment variable configuration with production validation.

use log::warn;

/// Get the PROXY_URL from environment or default
pub fn get_proxy_url() -> String {
    std::env::var("PROXY_URL").unwrap_or_else(|_| "http://localhost:8080".to_string())
}

/// Construct a full tunnel URL from subdomain and proxy URL
pub fn get_tunnel_url(subdomain: &str) -> String {
    let proxy_url = get_proxy_url();
    if let Some(stripped) = proxy_url.strip_prefix("http://") {
        let host = stripped.split('/').next().unwrap_or(stripped);
        format!("http://{}.{}", subdomain, host)
    } else if let Some(stripped) = proxy_url.strip_prefix("https://") {
        let host = stripped.split('/').next().unwrap_or(stripped);
        format!("https://{}.{}", subdomain, host)
    } else {
        format!("http://{}.{}", subdomain, proxy_url)
    }
}

/// Check if running in development mode
pub fn is_development() -> bool {
    match std::env::var("NODE_ENV").as_deref() {
        Ok("production") => false,
        Ok("prod") => false,
        _ => {
            // Also check RUST_ENV for Rust-native configuration
            match std::env::var("RUST_ENV").as_deref() {
                Ok("production") => false,
                Ok("prod") => false,
                _ => true,
            }
        }
    }
}

/// Validate critical configuration at startup
pub fn validate_config() -> Result<(), String> {
    let is_dev = is_development();

    // Check INTERNAL_API_SECRET
    let internal_secret = std::env::var("INTERNAL_API_SECRET").ok();
    match internal_secret.as_deref() {
        None if !is_dev => {
            return Err("INTERNAL_API_SECRET must be set in production".to_string());
        }
        Some("dev-secret") if !is_dev => {
            return Err("INTERNAL_API_SECRET cannot be 'dev-secret' in production".to_string());
        }
        Some(secret) if secret.len() < 32 && !is_dev => {
            return Err("INTERNAL_API_SECRET must be at least 32 characters in production".to_string());
        }
        None => {
            warn!("INTERNAL_API_SECRET not set, using default 'dev-secret' (development only)");
        }
        _ => {}
    }

    // Check API_BASE_URL
    if std::env::var("API_BASE_URL").is_err() && !is_dev {
        warn!("API_BASE_URL not set in production, using default 'http://localhost:3000'");
    }

    // Check TUNNL_SKIP_AUTH in production
    if std::env::var("TUNNL_SKIP_AUTH").is_ok() && !is_dev {
        return Err("TUNNL_SKIP_AUTH cannot be set in production".to_string());
    }

    Ok(())
}
