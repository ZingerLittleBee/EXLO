//! SSH handler for individual connections with Device Flow authentication.

use std::net::SocketAddr;
use std::sync::Arc;

use log::{debug, error, info, warn};
use russh::server::Handle;
use russh::ChannelId;
use tokio::sync::{oneshot, Mutex};

use crate::device::{generate_activation_code, DeviceFlowClient};
use crate::error::TunnelError;
use crate::state::AppState;
use crate::terminal_ui;

use super::tunnel::{create_tunnel, CreateTunnelResult};
use super::types::{
    generate_secure_subdomain_id, generate_session_id, SharedHandlerState, VerificationStatus,
};
use super::verification::spawn_verification_polling;

/// Handler for a single SSH connection.
pub struct SshHandler {
    pub(super) state: Arc<AppState>,
    pub(super) device_flow_client: Arc<DeviceFlowClient>,
    pub(super) peer_addr: Option<SocketAddr>,
    pub(super) username: Option<String>,
    pub(super) session_handle: Option<Handle>,
    pub(super) session_channel_id: Option<ChannelId>,
    pub(super) session_id: String,
    pub(super) poll_cancel: Option<oneshot::Sender<()>>,
    pub(super) shared_state: Arc<Mutex<SharedHandlerState>>,
    pub(super) public_key_fingerprint: Option<String>,
}

impl SshHandler {
    pub fn new(
        state: Arc<AppState>,
        device_flow_client: Arc<DeviceFlowClient>,
        peer_addr: Option<SocketAddr>,
    ) -> Self {
        let session_id = generate_session_id();
        let shared_state = Arc::new(Mutex::new(SharedHandlerState::new()));
        Self {
            state,
            device_flow_client,
            peer_addr,
            username: None,
            session_handle: None,
            session_channel_id: None,
            session_id,
            poll_cancel: None,
            shared_state,
            public_key_fingerprint: None,
        }
    }

    pub(super) async fn generate_subdomain(&self) -> String {
        let mut state = self.shared_state.lock().await;
        state.subdomain_counter += 1;
        let random_id = generate_secure_subdomain_id();
        format!("tunnel-{}-{}", random_id, state.subdomain_counter)
    }

    pub(super) async fn send_tunnel_message(&self, port: u32) {
        let (display_name, tunnels) = {
            let state = self.shared_state.lock().await;
            let display_name = match &state.verification_status {
                VerificationStatus::Verified { display_name, .. } => display_name.clone(),
                _ => "unknown".to_string(),
            };
            let tunnels: Vec<(String, u32)> = state
                .registered_subdomains
                .iter()
                .map(|s| (s.clone(), port))
                .collect();
            (display_name, tunnels)
        };

        if tunnels.is_empty() {
            return;
        }

        let message = terminal_ui::create_success_box(&display_name, &tunnels);

        info!(
            "send_tunnel_message: session_handle={}, session_channel_id={:?}",
            self.session_handle.is_some(),
            self.session_channel_id
        );

        if let (Some(handle), Some(channel_id)) = (&self.session_handle, self.session_channel_id) {
            info!("Sending tunnel message to channel {:?}", channel_id);
            if let Err(e) = handle
                .data(channel_id, message.into_bytes().into())
                .await
            {
                warn!("Failed to send tunnel message via session channel: {:?}", e);
            } else {
                info!("Tunnel message sent to client");
                return;
            }
        }

        // Session channel not ready yet, save for later
        {
            let mut state = self.shared_state.lock().await;
            state.pending_tunnel_port = Some(port);
            info!(
                "Session channel not ready, deferring tunnel message (port={})",
                port
            );
        }
    }

    pub(super) async fn cleanup_tunnels(&self) {
        let subdomains: Vec<String> = {
            let state = self.shared_state.lock().await;
            state.registered_subdomains.clone()
        };
        for subdomain in &subdomains {
            // Mark tunnel as disconnected instead of removing it
            // This allows the dashboard to show the correct status while keeping
            // the tunnel entry for the reconnection window
            self.state.mark_tunnel_disconnected(subdomain).await;
            info!("Marked tunnel as disconnected: {}", subdomain);
            
            // Also notify web server about disconnection
            if let Err(e) = self.device_flow_client.unregister_tunnel(subdomain).await {
                warn!("Failed to unregister tunnel from web server: {}", e);
            }
        }
        self.shared_state
            .lock()
            .await
            .registered_subdomains
            .clear();
    }

    pub(super) async fn is_verified(&self) -> bool {
        let state = self.shared_state.lock().await;
        matches!(
            state.verification_status,
            VerificationStatus::Verified { .. }
        )
    }

    pub(super) async fn get_verification_status(&self) -> VerificationStatus {
        self.shared_state.lock().await.verification_status.clone()
    }

    pub(super) async fn start_device_flow(&mut self) -> Result<String, String> {
        // Check rate limiting atomically
        if let Some(peer) = self.peer_addr {
            let ip = peer.ip();
            if self.state.check_and_record_device_flow(ip).await {
                let reason = "Rate limited: too many Device Flow requests. Please wait before trying again.".to_string();
                warn!("Device Flow rate limited for IP: {}", ip);
                {
                    let mut state = self.shared_state.lock().await;
                    state.verification_status = VerificationStatus::Failed {
                        reason: reason.clone(),
                    };
                }
                return Err(reason);
            }
        }

        let code = generate_activation_code();
        let session_id = self.session_id.clone();
        let client = self.device_flow_client.clone();

        debug!("Starting Device Flow with code: {}", code);

        match client.register_code(&code, &session_id).await {
            Ok(()) => {
                let activation_url = client.get_activation_url(&code);

                debug!(
                    "Device Flow started!\n\
                     Code: [REDACTED]\n\
                     URL: {}",
                    activation_url
                );

                {
                    let mut state = self.shared_state.lock().await;
                    state.verification_status = VerificationStatus::Pending { code: code.clone() };
                }

                let (cancel_tx, cancel_rx) = oneshot::channel();
                self.poll_cancel = Some(cancel_tx);

                spawn_verification_polling(
                    code.clone(),
                    session_id,
                    cancel_rx,
                    self.device_flow_client.clone(),
                    self.shared_state.clone(),
                    self.state.clone(),
                    self.peer_addr,
                    self.public_key_fingerprint.clone(),
                );

                Ok(code)
            }
            Err(e) => {
                let reason = format!("Failed to register code: {}", e);
                error!("{}", reason);
                {
                    let mut state = self.shared_state.lock().await;
                    state.verification_status = VerificationStatus::Failed {
                        reason: reason.clone(),
                    };
                }
                Err(reason)
            }
        }
    }

    pub(super) async fn do_create_tunnel(&self, address: &str, port: u32) -> Result<CreateTunnelResult, TunnelError> {
        create_tunnel(
            address,
            port,
            self.session_handle.as_ref(),
            &self.shared_state,
            &self.state,
            self.peer_addr,
            self.username.as_deref(),
            self.public_key_fingerprint.as_deref(),
            self.generate_subdomain(),
        )
        .await
    }
}

impl Drop for SshHandler {
    fn drop(&mut self) {
        // Cancel the polling task if it's still running
        if let Some(cancel) = self.poll_cancel.take() {
            let _ = cancel.send(());
        }
        
        // When the handler is dropped (connection closed), clean up tunnels
        let state = self.state.clone();
        let shared_state = self.shared_state.clone();
        let device_flow_client = self.device_flow_client.clone();
        
        // Spawn a task to clean up since Drop can't be async
        tokio::spawn(async move {
            let subdomains: Vec<String> = {
                let state = shared_state.lock().await;
                state.registered_subdomains.clone()
            };
            
            if subdomains.is_empty() {
                return;
            }
            
            info!("Handler dropped, marking {} tunnel(s) as disconnected", subdomains.len());
            
            for subdomain in &subdomains {
                state.mark_tunnel_disconnected(subdomain).await;
                if let Err(e) = device_flow_client.unregister_tunnel(subdomain).await {
                    warn!("Failed to unregister tunnel from web server: {}", e);
                }
            }
        });
    }
}
