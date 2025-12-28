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
    /// The subdomain that caused the conflict (for subdomain taken errors)
    pub conflicting_subdomain: Option<String>,
    /// Whether the conflict is from an explicit subdomain (should disconnect) or fallback (use random)
    pub is_explicit_conflict: bool,
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
            return Ok(CreateTunnelResult {
                success: false,
                conflicting_subdomain: None,
                is_explicit_conflict: false,
            });
        }
    };

    // Priority: 
    // 1. requested_subdomain (from username, strict - disconnect on conflict)
    //    - If username matches last_subdomain for this port, treat as reconnection
    // 2. last_subdomains (reconnection for "." username within 30min TTL)
    // 3. generate new random subdomain (when username is "." and no previous subdomain)
    let (subdomain, is_reconnect) = {
        let state = shared_state.lock().await;
        if let Some(ref requested) = state.requested_subdomain {
            // User explicitly specified a subdomain via username
            // Check if it matches a previous subdomain (reconnection)
            let is_reconnect = state.last_subdomains.get(&port).is_some_and(|last| last == requested);
            if is_reconnect {
                info!("Reconnecting with same subdomain for port {}: {}", port, requested);
            } else {
                info!("Using username as subdomain: {}", requested);
            }
            (requested.clone(), is_reconnect)
        } else if let Some(last) = state.last_subdomains.get(&port) {
            // User requested random (username "."), but has a previous subdomain within TTL
            // Reuse the previous subdomain for reconnection
            info!("Reconnecting with previous subdomain for port {} (username is '.'): {}", port, last);
            (last.clone(), true)
        } else {
            // User requested random and no previous subdomain
            drop(state);
            info!("Generating random subdomain (username is '.')");
            (generate_subdomain.await, false)
        }
    };

    // Determine if this is an explicit subdomain request (user specified via username)
    let is_explicit = {
        let state = shared_state.lock().await;
        state.requested_subdomain.is_some()
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
                let (user_id, display_name) = {
                    let state = shared_state.lock().await;
                    match &state.verification_status {
                        VerificationStatus::Verified { user_id, display_name } => {
                            (user_id.clone(), Some(display_name.clone()))
                        }
                        _ => (username.unwrap_or("anonymous").to_string(), None),
                    }
                };
                app_state
                    .save_verified_key(
                        fingerprint,
                        &user_id,
                        display_name.as_deref(),
                        port,
                        &subdomain,
                    )
                    .await;
            }
            
            Ok(CreateTunnelResult {
                success: true,
                conflicting_subdomain: None,
                is_explicit_conflict: false,
            })
        }
        Err(TunnelError::SubdomainTaken(s)) => {
            warn!("Subdomain {} already taken (explicit={})", s, is_explicit);
            Ok(CreateTunnelResult {
                success: false,
                conflicting_subdomain: Some(s),
                is_explicit_conflict: is_explicit,
            })
        }
        Err(e) => {
            error!("Failed to register tunnel: {}", e);
            Err(e)
        }
    }
}
