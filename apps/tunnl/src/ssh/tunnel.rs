//! Tunnel creation and management logic.

use std::sync::Arc;
use std::time::SystemTime;

use log::{error, info, warn};
use russh::server::Handle;
use tokio::sync::Mutex;

use crate::config::get_tunnel_url;
use crate::error::TunnelError;
use crate::state::{AppState, TunnelInfo};

use super::types::{SharedHandlerState, VerificationStatus};

/// Result of tunnel creation
#[derive(Debug, Clone)]
pub struct CreateTunnelResult {
    /// Whether the tunnel was created successfully
    pub success: bool,
}

/// Create a tunnel after verification
pub async fn create_tunnel(
    address: &str,
    port: u32,
    session_handle: Option<&Handle>,
    shared_state: &Arc<Mutex<SharedHandlerState>>,
    app_state: &Arc<AppState>,
    peer_addr: Option<std::net::SocketAddr>,
    username: Option<&str>,
    public_key_fingerprint: Option<&str>,
    generate_subdomain: impl std::future::Future<Output = String>,
) -> Result<CreateTunnelResult, TunnelError> {
    let handle = match session_handle {
        Some(h) => h.clone(),
        None => {
            error!("No session handle available!");
            return Ok(CreateTunnelResult { success: false });
        }
    };

    // Use last_subdomains if available for this port (reconnection), otherwise generate new one
    let (subdomain, is_reconnect) = {
        let state = shared_state.lock().await;
        if let Some(last) = state.last_subdomains.get(&port) {
            info!("Reusing subdomain from previous session for port {}: {}", port, last);
            (last.clone(), true)
        } else {
            drop(state);
            (generate_subdomain.await, false)
        }
    };

    // If reconnecting, remove the old tunnel first (stale from previous session)
    if is_reconnect {
        if let Ok(old_info) = app_state.remove_tunnel(&subdomain).await {
            info!(
                "Removed stale tunnel for reconnection: {} (was from {})",
                subdomain, old_info.client_ip
            );
        }
    }

    let tunnel_username = {
        let state = shared_state.lock().await;
        match &state.verification_status {
            VerificationStatus::Verified { user_id, .. } => user_id.clone(),
            _ => username.unwrap_or("anonymous").to_string(),
        }
    };

    let client_ip = peer_addr
        .map(|addr| addr.ip().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let tunnel_info = TunnelInfo {
        subdomain: subdomain.clone(),
        handle,
        requested_address: address.to_string(),
        requested_port: port,
        server_port: 80,
        created_at: SystemTime::now(),
        username: tunnel_username,
        client_ip,
        is_connected: true,
        disconnected_at: None,
    };

    match app_state.register_tunnel(tunnel_info).await {
        Ok(()) => {
            let tunnel_url = get_tunnel_url(&subdomain);
            info!(
                "✓ Tunnel registered!\n\
                 Subdomain: {}\n\
                 URL: {}",
                subdomain, tunnel_url
            );
            shared_state
                .lock()
                .await
                .registered_subdomains
                .push(subdomain.clone());
            // Store subdomain by port for future reconnections
            shared_state
                .lock()
                .await
                .last_subdomains
                .insert(port, subdomain.clone());
            
            // Save to verified_key for persistence across sessions
            if let Some(fingerprint) = public_key_fingerprint {
                let display_name = {
                    let state = shared_state.lock().await;
                    match &state.verification_status {
                        VerificationStatus::Verified { display_name, .. } => Some(display_name.clone()),
                        _ => None,
                    }
                };
                app_state
                    .save_verified_key(
                        fingerprint,
                        username.unwrap_or("anonymous"),
                        display_name.as_deref(),
                        port,
                        &subdomain,
                    )
                    .await;
            }
            
            Ok(CreateTunnelResult { success: true })
        }
        Err(TunnelError::SubdomainTaken(s)) => {
            warn!("Subdomain {} already taken", s);
            Ok(CreateTunnelResult { success: false })
        }
        Err(e) => {
            error!("Failed to register tunnel: {}", e);
            Err(e)
        }
    }
}
