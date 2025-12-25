//! SSH handler for individual connections with Device Flow authentication.

use std::net::SocketAddr;
use std::sync::Arc;

use async_trait::async_trait;
use log::{debug, error, info, warn};
use russh::keys::PublicKey;
use russh::server::{Auth, Handle, Handler, Msg, Session};
use russh::{Channel, ChannelId, Disconnect};
use russh_keys::HashAlg;
use tokio::sync::{oneshot, Mutex};

use crate::config::is_development;
use crate::device::{generate_activation_code, DeviceFlowClient};
use crate::error::TunnelError;
use crate::state::AppState;
use crate::terminal_ui;

use super::tunnel::create_tunnel;
use super::types::{
    generate_session_id, rand_simple, PendingTunnel, SharedHandlerState, VerificationStatus,
};
use super::verification::spawn_verification_polling;

/// Handler for a single SSH connection.
pub struct SshHandler {
    state: Arc<AppState>,
    device_flow_client: Arc<DeviceFlowClient>,
    peer_addr: Option<SocketAddr>,
    username: Option<String>,
    session_handle: Option<Handle>,
    session_channel_id: Option<ChannelId>,
    session_id: String,
    poll_cancel: Option<oneshot::Sender<()>>,
    shared_state: Arc<Mutex<SharedHandlerState>>,
    public_key_fingerprint: Option<String>,
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

    async fn generate_subdomain(&self) -> String {
        let mut state = self.shared_state.lock().await;
        state.subdomain_counter += 1;
        let random_part: u32 = rand_simple();
        format!("tunnel-{:06x}-{}", random_part, state.subdomain_counter)
    }

    async fn send_reconnect_message(&self, port: u32) {
        let (user_id, tunnels) = {
            let state = self.shared_state.lock().await;
            let user_id = match &state.verification_status {
                VerificationStatus::Verified { user_id } => user_id.clone(),
                _ => "unknown".to_string(),
            };
            let tunnels: Vec<(String, u32)> = state
                .registered_subdomains
                .iter()
                .map(|s| (s.clone(), port))
                .collect();
            (user_id, tunnels)
        };

        if tunnels.is_empty() {
            return;
        }

        let message = terminal_ui::create_reconnect_box(&user_id, &tunnels);

        info!(
            "send_reconnect_message: session_handle={}, session_channel_id={:?}",
            self.session_handle.is_some(),
            self.session_channel_id
        );

        if let (Some(handle), Some(channel_id)) = (&self.session_handle, self.session_channel_id) {
            info!("Sending reconnect message to channel {:?}", channel_id);
            if let Err(e) = handle
                .data(channel_id, message.into_bytes().into())
                .await
            {
                warn!("Failed to send reconnect message via session channel: {:?}", e);
            } else {
                info!("Reconnect message sent to client");
                return;
            }
        }

        // Session channel not ready yet, save port for later
        {
            let mut state = self.shared_state.lock().await;
            state.pending_reconnect_port = Some(port);
            info!(
                "Session channel not ready, deferring reconnect message (port={})",
                port
            );
        }
    }

    async fn cleanup_tunnels(&self) {
        let subdomains: Vec<String> = {
            let state = self.shared_state.lock().await;
            state.registered_subdomains.clone()
        };
        for subdomain in &subdomains {
            match self.state.remove_tunnel(subdomain).await {
                Ok(_) => {
                    info!("Removed tunnel: {}", subdomain);
                    if let Err(e) = self.device_flow_client.unregister_tunnel(subdomain).await {
                        warn!("Failed to unregister tunnel from web server: {}", e);
                    }
                }
                Err(e) => warn!("Failed to remove tunnel {}: {}", subdomain, e),
            }
        }
        self.shared_state
            .lock()
            .await
            .registered_subdomains
            .clear();
    }

    async fn is_verified(&self) -> bool {
        let state = self.shared_state.lock().await;
        matches!(
            state.verification_status,
            VerificationStatus::Verified { .. }
        )
    }

    async fn get_verification_status(&self) -> VerificationStatus {
        self.shared_state.lock().await.verification_status.clone()
    }

    async fn start_device_flow(&mut self) -> Result<String, String> {
        let code = generate_activation_code();
        let session_id = self.session_id.clone();
        let client = self.device_flow_client.clone();

        info!("Starting Device Flow with code: {}", code);

        match client.register_code(&code, &session_id).await {
            Ok(()) => {
                let activation_url = client.get_activation_url(&code);

                info!(
                    "Device Flow started!\n\
                     Code: {}\n\
                     URL: {}",
                    code, activation_url
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

    async fn do_create_tunnel(&self, address: &str, port: u32) -> Result<bool, TunnelError> {
        create_tunnel(
            address,
            port,
            self.session_handle.as_ref(),
            &self.shared_state,
            &self.state,
            self.peer_addr,
            self.username.as_deref(),
            self.generate_subdomain(),
        )
        .await
    }
}

#[async_trait]
impl Handler for SshHandler {
    type Error = TunnelError;

    async fn auth_succeeded(&mut self, session: &mut Session) -> Result<(), Self::Error> {
        info!("Authentication succeeded for user: {:?}", self.username);
        let handle = session.handle();
        self.session_handle = Some(handle.clone());
        self.shared_state.lock().await.session_handle = Some(handle);
        Ok(())
    }

    async fn channel_close(
        &mut self,
        channel: ChannelId,
        _session: &mut Session,
    ) -> Result<(), Self::Error> {
        if self.session_channel_id == Some(channel) {
            info!("Session channel {:?} closed, cleaning up...", channel);

            if let Some(cancel) = self.poll_cancel.take() {
                let _ = cancel.send(());
            }

            self.cleanup_tunnels().await;
        } else {
            debug!("Forwarded channel {:?} closed", channel);
        }

        Ok(())
    }

    async fn auth_publickey(
        &mut self,
        user: &str,
        public_key: &PublicKey,
    ) -> Result<Auth, Self::Error> {
        let fingerprint = public_key.fingerprint(HashAlg::Sha256);

        info!(
            "Public key auth attempt: user='{}', fingerprint='{}'",
            user, fingerprint
        );

        self.username = Some(user.to_string());
        let fingerprint_str = fingerprint.to_string();
        self.public_key_fingerprint = Some(fingerprint_str.clone());

        if let Some(verified_key) = self.state.get_verified_key(&fingerprint_str).await {
            info!(
                "Public key already verified for user '{}', subdomain={:?}, skipping Device Flow",
                verified_key.user_id, verified_key.last_subdomain
            );
            let mut state = self.shared_state.lock().await;
            state.verification_status = VerificationStatus::Verified {
                user_id: verified_key.user_id,
            };
            state.last_subdomain = verified_key.last_subdomain;
        }

        Ok(Auth::Accept)
    }

    async fn tcpip_forward(
        &mut self,
        address: &str,
        port: &mut u32,
        _session: &mut Session,
    ) -> Result<bool, Self::Error> {
        let status = self.get_verification_status().await;
        info!(
            "=== Tunnel Request ===\n\
             Address: '{}'\n\
             Port: {}\n\
             User: {:?}\n\
             Status: {:?}",
            address, port, self.username, status
        );

        // Skip auth completely if TUNNL_SKIP_AUTH is set (development only)
        if std::env::var("TUNNL_SKIP_AUTH").is_ok() && is_development() {
            if !self.is_verified().await {
                warn!("TUNNL_SKIP_AUTH is set - bypassing Device Flow verification (development mode)");
                let mut state = self.shared_state.lock().await;
                state.verification_status = VerificationStatus::Verified {
                    user_id: self.username.clone().unwrap_or_else(|| "dev".to_string()),
                };
            }
            return self.do_create_tunnel(address, *port).await;
        }

        // If already verified (reconnection), create tunnel immediately
        if self.is_verified().await {
            let result = self.do_create_tunnel(address, *port).await?;
            if result {
                self.send_reconnect_message(*port).await;
            }
            return Ok(result);
        }

        // Store the tunnel request as pending
        {
            let mut state = self.shared_state.lock().await;
            state.pending_tunnels.push(PendingTunnel {
                address: address.to_string(),
                port: *port,
            });
            info!(
                "Tunnel request stored as pending (total: {})",
                state.pending_tunnels.len()
            );
        }

        // Start Device Flow if not already started
        let status = self.get_verification_status().await;
        if matches!(status, VerificationStatus::NotStarted) {
            match self.start_device_flow().await {
                Ok(code) => {
                    let url = self.device_flow_client.get_activation_url(&code);
                    info!("Device Flow started - Code: {}, URL: {}", code, url);
                }
                Err(reason) => {
                    warn!("Device Flow failed: {}", reason);
                    return Ok(false);
                }
            }
        }

        Ok(true)
    }

    async fn cancel_tcpip_forward(
        &mut self,
        address: &str,
        port: u32,
        _session: &mut Session,
    ) -> Result<bool, Self::Error> {
        info!("Cancel tcpip_forward: address='{}', port={}", address, port);

        let tunnels_to_remove: Vec<String> = {
            let state = self.shared_state.lock().await;
            state.registered_subdomains.clone()
        };

        for subdomain in tunnels_to_remove {
            if let Ok(info) = self.state.remove_tunnel(&subdomain).await {
                if info.requested_address == address && info.requested_port == port {
                    let mut state = self.shared_state.lock().await;
                    state.registered_subdomains.retain(|s| s != &subdomain);
                    info!("Removed tunnel: {}", subdomain);
                }
            }
        }

        Ok(true)
    }

    async fn channel_open_session(
        &mut self,
        channel: Channel<Msg>,
        session: &mut Session,
    ) -> Result<bool, Self::Error> {
        let channel_id = channel.id();
        info!("Session channel opened: id={:?}", channel_id);
        self.session_channel_id = Some(channel_id);
        self.shared_state.lock().await.session_channel_id = Some(channel_id);

        // Check if there's a pending reconnect message from tcpip_forward
        let pending_port = {
            let mut state = self.shared_state.lock().await;
            state.pending_reconnect_port.take()
        };

        // Note: pending_reconnect_port will be handled in shell_request
        if pending_port.is_some() {
            let mut state = self.shared_state.lock().await;
            state.pending_reconnect_port = pending_port;
        }

        // Check verification status for new connections
        let status = self.get_verification_status().await;

        match status {
            VerificationStatus::Verified { ref user_id } => {
                let tunnels: Vec<(String, u32)> = {
                    let state = self.shared_state.lock().await;
                    let port = state.pending_tunnels.first().map(|t| t.port).unwrap_or(0);
                    if !state.registered_subdomains.is_empty() {
                        state
                            .registered_subdomains
                            .iter()
                            .map(|s| (s.clone(), port))
                            .collect()
                    } else if let Some(ref last) = state.last_subdomain {
                        vec![(last.clone(), port)]
                    } else {
                        Vec::new()
                    }
                };

                if !tunnels.is_empty() {
                    let message = terminal_ui::create_reconnect_box(user_id, &tunnels);
                    if let Err(e) = session.data(channel_id, message.into_bytes().into()) {
                        warn!("Failed to send reconnect message: {:?}", e);
                    }
                }
            }
            VerificationStatus::NotStarted => {
                match self.start_device_flow().await {
                    Ok(code) => {
                        let url = self.device_flow_client.get_activation_url(&code);
                        info!("Device Flow started - Code: {}, URL: {}", code, url);

                        let message = terminal_ui::create_activation_box(&code, &url);
                        if let Err(e) = session.data(channel_id, message.into_bytes().into()) {
                            warn!("Failed to send activation message: {:?}", e);
                        }
                    }
                    Err(reason) => {
                        warn!("Device Flow failed to start: {}", reason);
                    }
                }
            }
            _ => {}
        }

        Ok(true)
    }

    async fn data(
        &mut self,
        channel: ChannelId,
        data: &[u8],
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        debug!(
            "Data received on channel {:?}: {} bytes",
            channel,
            data.len()
        );

        if data.contains(&27) {
            let mut state = self.shared_state.lock().await;
            let now = std::time::Instant::now();

            if state.esc_pressed {
                if let Some(last_time) = state.last_esc_time {
                    if now.duration_since(last_time).as_secs() < 2 {
                        drop(state);
                        info!("Double ESC detected, disconnecting...");
                        if let Some(handle) = &self.session_handle {
                            let _ = handle
                                .disconnect(
                                    Disconnect::ByApplication,
                                    "Disconnected by user".to_string(),
                                    "en".to_string(),
                                )
                                .await;
                        }
                        return Ok(());
                    }
                }
            }

            state.esc_pressed = true;
            state.last_esc_time = Some(now);
            drop(state);

            let hint = terminal_ui::create_esc_hint();
            session.data(channel, hint.into_bytes().into())?;

            let shared_state = self.shared_state.clone();
            let handle = self.session_handle.clone();
            let channel_id = channel;
            tokio::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                let mut state = shared_state.lock().await;
                if state.esc_pressed {
                    state.esc_pressed = false;
                    state.last_esc_time = None;
                    if let Some(h) = handle {
                        let clear = terminal_ui::clear_esc_hint();
                        let _ = h.data(channel_id, clear.into_bytes().into()).await;
                    }
                }
            });

            return Ok(());
        }

        Ok(())
    }

    async fn channel_eof(
        &mut self,
        channel: ChannelId,
        _session: &mut Session,
    ) -> Result<(), Self::Error> {
        debug!("EOF on channel {:?}", channel);
        Ok(())
    }

    async fn pty_request(
        &mut self,
        channel: ChannelId,
        _term: &str,
        _col_width: u32,
        _row_height: u32,
        _pix_width: u32,
        _pix_height: u32,
        _modes: &[(russh::Pty, u32)],
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        info!("PTY request on channel {:?}", channel);
        session.channel_success(channel)?;
        Ok(())
    }

    async fn shell_request(
        &mut self,
        channel: ChannelId,
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        info!("Shell request on channel {:?}", channel);
        session.channel_success(channel)?;

        // Check if there's a pending reconnect message
        let pending_port = {
            let mut state = self.shared_state.lock().await;
            state.pending_reconnect_port.take()
        };

        if let Some(port) = pending_port {
            let (user_id, tunnels) = {
                let state = self.shared_state.lock().await;
                let user_id = match &state.verification_status {
                    VerificationStatus::Verified { user_id } => user_id.clone(),
                    _ => "unknown".to_string(),
                };
                let tunnels: Vec<(String, u32)> = state
                    .registered_subdomains
                    .iter()
                    .map(|s| (s.clone(), port))
                    .collect();
                (user_id, tunnels)
            };

            if !tunnels.is_empty() {
                let message = terminal_ui::create_reconnect_box(&user_id, &tunnels);
                if let Err(e) = session.data(channel, message.into_bytes().into()) {
                    warn!("Failed to send reconnect message in shell_request: {:?}", e);
                } else {
                    info!("Reconnect message sent in shell_request");
                }
            }
            return Ok(());
        }

        // Send the activation message if Device Flow is pending
        let status = self.get_verification_status().await;
        if let VerificationStatus::Pending { code } = status {
            let url = self.device_flow_client.get_activation_url(&code);
            let message = terminal_ui::create_activation_box(&code, &url);
            if let Err(e) = session.data(channel, message.into_bytes().into()) {
                warn!("Failed to send activation message: {:?}", e);
            }
        }

        Ok(())
    }
}
