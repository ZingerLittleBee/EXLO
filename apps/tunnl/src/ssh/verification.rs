//! Device Flow verification polling logic.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::SystemTime;

use log::{error, info, warn};
use russh::Disconnect;
use tokio::sync::{oneshot, Mutex};

use crate::device::{DeviceFlowClient, RegisterTunnelRequest, VerifiedUser};
use crate::state::{AppState, TunnelInfo};
use crate::terminal_ui;

use super::types::{generate_secure_subdomain_id, PendingTunnel, SharedHandlerState, VerificationStatus};

/// Spawn a background task to poll for Device Flow verification
pub fn spawn_verification_polling(
    code: String,
    session_id: String,
    cancel_rx: oneshot::Receiver<()>,
    client: Arc<DeviceFlowClient>,
    shared_state: Arc<Mutex<SharedHandlerState>>,
    app_state: Arc<AppState>,
    peer_addr: Option<SocketAddr>,
    public_key_fingerprint: Option<String>,
) {
    tokio::spawn(async move {
        let mut frame_idx = 0;

        // Spawn a task to animate the spinner
        let shared_state_clone = shared_state.clone();
        let spinner_handle = tokio::spawn(async move {
            loop {
                let (handle, channel_id) = {
                    let state = shared_state_clone.lock().await;
                    (state.session_handle.clone(), state.session_channel_id)
                };

                if let (Some(handle), Some(channel_id)) = (handle, channel_id) {
                    let update = terminal_ui::create_spinner_update(frame_idx);
                    let _ = handle.data(channel_id, update.into_bytes().into()).await;
                }

                frame_idx += 1;
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        });

        tokio::select! {
            result = client.poll_until_verified(&code) => {
                spinner_handle.abort();
                handle_verification_result(
                    result,
                    shared_state,
                    app_state,
                    client,
                    session_id,
                    peer_addr,
                    public_key_fingerprint,
                ).await;
            }
            _ = cancel_rx => {
                spinner_handle.abort();
                info!("Verification polling cancelled");
            }
        }
    });
}

async fn handle_verification_result(
    result: Result<VerifiedUser, anyhow::Error>,
    shared_state: Arc<Mutex<SharedHandlerState>>,
    app_state: Arc<AppState>,
    client: Arc<DeviceFlowClient>,
    session_id: String,
    peer_addr: Option<SocketAddr>,
    public_key_fingerprint: Option<String>,
) {
    match result {
        Ok(verified_user) => {
            info!("Device Flow verified! User ID: {}", verified_user.user_id);
            handle_verification_success(
                verified_user,
                shared_state,
                app_state,
                client,
                session_id,
                peer_addr,
                public_key_fingerprint,
            )
            .await;
        }
        Err(e) => {
            let reason = format!("{}", e);
            error!("Verification failed: {}", reason);
            handle_verification_failure(reason, shared_state).await;
        }
    }
}

async fn handle_verification_success(
    verified_user: VerifiedUser,
    shared_state: Arc<Mutex<SharedHandlerState>>,
    app_state: Arc<AppState>,
    client: Arc<DeviceFlowClient>,
    session_id: String,
    peer_addr: Option<SocketAddr>,
    public_key_fingerprint: Option<String>,
) {
    let user_id = verified_user.user_id.clone();
    let display_name = verified_user.display_name();

    let (session_handle, session_channel_id, pending_tunnels) = {
        let mut state = shared_state.lock().await;
        state.verification_status = VerificationStatus::Verified {
            user_id: user_id.clone(),
            display_name: display_name.clone(),
        };
        (
            state.session_handle.clone(),
            state.session_channel_id,
            std::mem::take(&mut state.pending_tunnels),
        )
    };

    let handle = match session_handle {
        Some(h) => h,
        None => {
            error!("No session handle available for creating tunnels");
            return;
        }
    };

    let client_ip = peer_addr
        .map(|addr| addr.ip().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let created_tunnels = create_pending_tunnels(
        pending_tunnels,
        &handle,
        &user_id,
        &display_name,
        &client_ip,
        session_channel_id,
        &shared_state,
        &app_state,
        &client,
        &session_id,
        public_key_fingerprint.as_deref(),
    )
    .await;

    // Send success message to SSH client
    if let Some(channel_id) = session_channel_id {
        let success_msg = terminal_ui::create_success_box(&display_name, &created_tunnels);
        if let Err(e) = handle
            .data(channel_id, success_msg.into_bytes().into())
            .await
        {
            warn!("Failed to send tunnel success message: {:?}", e);
        }
    }
}

async fn handle_verification_failure(reason: String, shared_state: Arc<Mutex<SharedHandlerState>>) {
    let (session_handle, session_channel_id) = {
        let mut state = shared_state.lock().await;
        state.verification_status = VerificationStatus::Failed {
            reason: reason.clone(),
        };
        (state.session_handle.clone(), state.session_channel_id)
    };

    if let (Some(handle), Some(channel_id)) = (session_handle, session_channel_id) {
        let error_msg = terminal_ui::create_error_box(&reason);
        if let Err(e) = handle
            .data(channel_id, error_msg.into_bytes().into())
            .await
        {
            warn!("Failed to send error message: {:?}", e);
        }

        tokio::time::sleep(std::time::Duration::from_secs(3)).await;

        if let Err(e) = handle
            .disconnect(Disconnect::ByApplication, reason, "en".to_string())
            .await
        {
            warn!("Failed to disconnect session: {:?}", e);
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn create_pending_tunnels(
    pending_tunnels: Vec<PendingTunnel>,
    handle: &russh::server::Handle,
    user_id: &str,
    display_name: &str,
    client_ip: &str,
    session_channel_id: Option<russh::ChannelId>,
    shared_state: &Arc<Mutex<SharedHandlerState>>,
    app_state: &Arc<AppState>,
    client: &Arc<DeviceFlowClient>,
    session_id: &str,
    public_key_fingerprint: Option<&str>,
) -> Vec<(String, u32)> {
    let mut created_tunnels = Vec::new();

    for pending in pending_tunnels {
        let subdomain = {
            let mut state = shared_state.lock().await;
            state.subdomain_counter += 1;
            let random_id = generate_secure_subdomain_id();
            format!("tunnel-{}-{}", random_id, state.subdomain_counter)
        };

        // Probe the local port before registering the tunnel
        let probe_result = handle
            .channel_open_forwarded_tcpip(&pending.address, pending.port, "127.0.0.1", 12345)
            .await;

        match probe_result {
            Ok(channel) => {
                drop(channel);
                info!(
                    "Port probe succeeded for {}:{}",
                    pending.address, pending.port
                );
            }
            Err(e) => {
                warn!(
                    "Port probe failed for {}:{}: {:?}",
                    pending.address, pending.port, e
                );

                if let Some(channel_id) = session_channel_id {
                    let error_msg =
                        terminal_ui::create_port_error_box(pending.port, &pending.address);
                    let _ = handle
                        .data(channel_id, error_msg.into_bytes().into())
                        .await;
                }

                tokio::time::sleep(std::time::Duration::from_secs(3)).await;

                let reason = format!(
                    "Local service not available on {}:{}",
                    pending.address, pending.port
                );
                let _ = handle
                    .disconnect(Disconnect::ByApplication, reason, "en".to_string())
                    .await;
                return created_tunnels;
            }
        }

        let tunnel_info = TunnelInfo {
            subdomain: subdomain.clone(),
            handle: handle.clone(),
            requested_address: pending.address.clone(),
            requested_port: pending.port,
            server_port: 80,
            created_at: SystemTime::now(),
            username: user_id.to_string(),
            client_ip: client_ip.to_string(),
            is_connected: true,
            disconnected_at: None,
        };

        match app_state.register_tunnel(tunnel_info).await {
            Ok(()) => {
                let tunnel_url = crate::config::get_tunnel_url(&subdomain);
                info!(
                    "✓ Tunnel registered!\n\
                     Subdomain: {}\n\
                     URL: {}",
                    subdomain, tunnel_url
                );
                {
                    let mut state = shared_state.lock().await;
                    state.registered_subdomains.push(subdomain.clone());
                    // Set last_subdomain for future reconnections
                    state.last_subdomain = Some(subdomain.clone());
                }
                created_tunnels.push((subdomain.clone(), pending.port));

                // Save verified key with subdomain for reconnection
                if let Some(fingerprint) = public_key_fingerprint {
                    app_state
                        .save_verified_key(fingerprint, user_id, Some(display_name), Some(&subdomain))
                        .await;
                }

                // Register tunnel with web server for tracking
                let register_req = RegisterTunnelRequest {
                    subdomain: subdomain.clone(),
                    user_id: user_id.to_string(),
                    session_id: session_id.to_string(),
                    requested_address: pending.address.clone(),
                    requested_port: pending.port,
                    server_port: 80,
                    client_ip: client_ip.to_string(),
                };
                if let Err(e) = client.register_tunnel(&register_req).await {
                    warn!("Failed to register tunnel with web server: {}", e);
                }
            }
            Err(e) => {
                error!("Failed to register tunnel: {}", e);
            }
        }
    }

    created_tunnels
}
