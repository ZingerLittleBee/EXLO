//! SSH handler for individual connections with Device Flow authentication.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use log::{debug, error, info, warn};
use russh::keys::PublicKey;
use russh::server::{Auth, Handle, Handler, Msg, Session};
use russh::{Channel, ChannelId, Disconnect};
use russh_keys::HashAlg;
use tokio::sync::{oneshot, Mutex};

use crate::config::{get_tunnel_url, is_development};
use crate::device::{DeviceFlowClient, RegisterTunnelRequest, generate_activation_code};
use crate::error::TunnelError;
use crate::state::{AppState, TunnelInfo};
use crate::terminal_ui;

/// A pending tunnel request waiting for verification
#[derive(Debug, Clone)]
struct PendingTunnel {
    address: String,
    port: u32,
}

/// Shared state that can be accessed from the polling task
struct SharedHandlerState {
    verification_status: VerificationStatus,
    pending_tunnels: Vec<PendingTunnel>,
    registered_subdomains: Vec<String>,
    subdomain_counter: u32,
    /// Session handle for sending data to client (set after auth succeeds)
    session_handle: Option<Handle>,
    /// Session channel ID (set when session channel is opened)
    session_channel_id: Option<ChannelId>,
    /// Whether ESC was pressed once (for double-ESC to disconnect)
    esc_pressed: bool,
    /// Timestamp of last ESC press for timeout
    last_esc_time: Option<std::time::Instant>,
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

/// Handler for a single SSH connection.
pub struct SshHandler {
    state: Arc<AppState>,
    device_flow_client: Arc<DeviceFlowClient>,
    peer_addr: Option<SocketAddr>,
    username: Option<String>,
    session_handle: Option<Handle>,
    session_channel_id: Option<ChannelId>,
    /// Session ID for this connection
    session_id: String,
    /// Channel to cancel polling task
    poll_cancel: Option<oneshot::Sender<()>>,
    /// Shared state accessible from polling task
    shared_state: Arc<Mutex<SharedHandlerState>>,
}

impl SshHandler {
    pub fn new(
        state: Arc<AppState>,
        device_flow_client: Arc<DeviceFlowClient>,
        peer_addr: Option<SocketAddr>,
    ) -> Self {
        let session_id = generate_session_id();
        let shared_state = Arc::new(Mutex::new(SharedHandlerState {
            verification_status: VerificationStatus::NotStarted,
            pending_tunnels: Vec::new(),
            registered_subdomains: Vec::new(),
            subdomain_counter: 0,
            session_handle: None,
            session_channel_id: None,
            esc_pressed: false,
            last_esc_time: None,
        }));
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
        }
    }

    async fn generate_subdomain(&self) -> String {
        let mut state = self.shared_state.lock().await;
        state.subdomain_counter += 1;
        let random_part: u32 = rand_simple();
        format!("tunnel-{:06x}-{}", random_part, state.subdomain_counter)
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
                    // Unregister from web server
                    if let Err(e) = self.device_flow_client.unregister_tunnel(subdomain).await {
                        warn!("Failed to unregister tunnel from web server: {}", e);
                    }
                }
                Err(e) => warn!("Failed to remove tunnel {}: {}", subdomain, e),
            }
        }
        self.shared_state.lock().await.registered_subdomains.clear();
    }

    async fn is_verified(&self) -> bool {
        let state = self.shared_state.lock().await;
        matches!(state.verification_status, VerificationStatus::Verified { .. })
    }

    async fn get_verification_status(&self) -> VerificationStatus {
        self.shared_state.lock().await.verification_status.clone()
    }

    /// Start the Device Flow verification process
    /// Returns Ok(code) if started successfully, Err if failed
    async fn start_device_flow(&mut self) -> Result<String, String> {
        let code = generate_activation_code();
        let session_id = self.session_id.clone();
        let client = self.device_flow_client.clone();

        info!("Starting Device Flow with code: {}", code);

        // Register the code with the web server
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

                // Start polling cancellation channel
                let (cancel_tx, cancel_rx) = oneshot::channel();
                self.poll_cancel = Some(cancel_tx);

                // Start the polling task
                self.spawn_verification_polling(code.clone(), session_id.clone(), cancel_rx);

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

    /// Spawn a background task to poll for verification
    fn spawn_verification_polling(&self, code: String, session_id: String, cancel_rx: oneshot::Receiver<()>) {
        let client = self.device_flow_client.clone();
        let shared_state = self.shared_state.clone();
        let app_state = self.state.clone();
        let peer_addr = self.peer_addr;

        tokio::spawn(async move {
            let mut frame_idx = 0;

            // Spawn a task to animate the spinner
            let shared_state_clone = shared_state.clone();
            let spinner_handle = tokio::spawn(async move {
                loop {
                    // Get handle and channel from shared state
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

            // Use select to handle cancellation
            tokio::select! {
                result = client.poll_until_verified(&code) => {
                    // Stop the spinner
                    spinner_handle.abort();
                    match result {
                        Ok(user_id) => {
                            info!("Device Flow verified! User ID: {}", user_id);

                            // Get session handle and channel ID from shared state
                            // (they are set after the polling task is spawned)
                            let (session_handle, session_channel_id, pending_tunnels) = {
                                let mut state = shared_state.lock().await;
                                state.verification_status = VerificationStatus::Verified { user_id: user_id.clone() };
                                (
                                    state.session_handle.clone(),
                                    state.session_channel_id,
                                    std::mem::take(&mut state.pending_tunnels),
                                )
                            };

                            // Create all pending tunnels
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

                            let mut created_tunnels = Vec::new();

                            for pending in pending_tunnels {
                                let subdomain = {
                                    let mut state = shared_state.lock().await;
                                    state.subdomain_counter += 1;
                                    let random_part: u32 = rand_simple();
                                    format!("tunnel-{:06x}-{}", random_part, state.subdomain_counter)
                                };

                                // Probe the local port before registering the tunnel
                                let probe_result = handle
                                    .channel_open_forwarded_tcpip(
                                        &pending.address,
                                        pending.port,
                                        "127.0.0.1",
                                        12345,
                                    )
                                    .await;

                                match probe_result {
                                    Ok(channel) => {
                                        // Close the probe channel immediately
                                        drop(channel);
                                        info!("Port probe succeeded for {}:{}", pending.address, pending.port);
                                    }
                                    Err(e) => {
                                        warn!("Port probe failed for {}:{}: {:?}", pending.address, pending.port, e);

                                        // Send error message to SSH client
                                        if let Some(channel_id) = session_channel_id {
                                            let error_msg = terminal_ui::create_port_error_box(pending.port, &pending.address);
                                            let _ = handle.data(channel_id, error_msg.into_bytes().into()).await;
                                        }

                                        // Wait 3 seconds before disconnecting
                                        tokio::time::sleep(std::time::Duration::from_secs(3)).await;

                                        // Disconnect the session
                                        let reason = format!("Local service not available on {}:{}", pending.address, pending.port);
                                        let _ = handle.disconnect(Disconnect::ByApplication, reason, "en".to_string()).await;
                                        return;
                                    }
                                }

                                let tunnel_info = TunnelInfo {
                                    subdomain: subdomain.clone(),
                                    handle: handle.clone(),
                                    requested_address: pending.address.clone(),
                                    requested_port: pending.port,
                                    server_port: 80,
                                    created_at: Instant::now(),
                                    username: user_id.clone(),
                                    client_ip: client_ip.clone(),
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
                                        shared_state.lock().await.registered_subdomains.push(subdomain.clone());
                                        created_tunnels.push((subdomain.clone(), pending.port));

                                        // Register tunnel with web server for tracking
                                        let register_req = RegisterTunnelRequest {
                                            subdomain: subdomain.clone(),
                                            user_id: user_id.clone(),
                                            session_id: session_id.clone(),
                                            requested_address: pending.address.clone(),
                                            requested_port: pending.port,
                                            server_port: 80,
                                            client_ip: client_ip.clone(),
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

                            // Send success message to SSH client by replacing the activation box
                            if let Some(channel_id) = session_channel_id {
                                let success_msg = terminal_ui::create_success_box(&user_id, &created_tunnels);
                                if let Err(e) = handle.data(channel_id, success_msg.into_bytes().into()).await {
                                    warn!("Failed to send tunnel success message: {:?}", e);
                                }
                            }
                        }
                        Err(e) => {
                            let reason = format!("{}", e);
                            error!("Verification failed: {}", reason);

                            // Get session handle and channel ID from shared state
                            let (session_handle, session_channel_id) = {
                                let mut state = shared_state.lock().await;
                                state.verification_status = VerificationStatus::Failed { reason: reason.clone() };
                                (state.session_handle.clone(), state.session_channel_id)
                            };

                            // Send error message to SSH client by replacing the activation box
                            if let (Some(handle), Some(channel_id)) = (session_handle, session_channel_id) {
                                let error_msg = terminal_ui::create_error_box(&reason);
                                if let Err(e) = handle.data(channel_id, error_msg.into_bytes().into()).await {
                                    warn!("Failed to send error message: {:?}", e);
                                }

                                // Wait 3 seconds before disconnecting
                                tokio::time::sleep(std::time::Duration::from_secs(3)).await;

                                // Disconnect the session
                                if let Err(e) = handle.disconnect(Disconnect::ByApplication, reason, "en".to_string()).await {
                                    warn!("Failed to disconnect session: {:?}", e);
                                }
                            }
                        }
                    }
                }
                _ = cancel_rx => {
                    spinner_handle.abort();
                    info!("Verification polling cancelled");
                }
            }
        });
    }
}

fn generate_session_id() -> String {
    use std::time::SystemTime;
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("ssh-{:x}", now)
}

fn rand_simple() -> u32 {
    use std::time::SystemTime;
    let duration = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    ((duration.as_nanos() as u64 ^ 0x5DEECE66D) & 0xFFFFFF) as u32
}

#[async_trait]
impl Handler for SshHandler {
    type Error = TunnelError;

    async fn auth_succeeded(&mut self, session: &mut Session) -> Result<(), Self::Error> {
        info!("Authentication succeeded for user: {:?}", self.username);
        let handle = session.handle();
        self.session_handle = Some(handle.clone());
        // Also store in shared state for the polling task
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
            return self.create_tunnel(address, *port).await;
        }

        // If already verified, create tunnel immediately
        if self.is_verified().await {
            return self.create_tunnel(address, *port).await;
        }

        // Store the tunnel request as pending - it will be created after verification
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

        // Return true to tell SSH client the forward is "accepted"
        // The actual tunnel will be created after verification completes
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
        // Also store in shared state for the polling task
        self.shared_state.lock().await.session_channel_id = Some(channel_id);

        // Start Device Flow if not already started or verified
        let status = self.get_verification_status().await;
        if matches!(status, VerificationStatus::NotStarted) {
            match self.start_device_flow().await {
                Ok(code) => {
                    let url = self.device_flow_client.get_activation_url(&code);

                    // Log to server
                    info!("Device Flow started - Code: {}, URL: {}", code, url);

                    // Send activation box to client
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

        Ok(true)
    }

    async fn data(
        &mut self,
        channel: ChannelId,
        data: &[u8],
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        debug!("Data received on channel {:?}: {} bytes", channel, data.len());

        // Check for ESC key (ASCII 27)
        if data.contains(&27) {
            let mut state = self.shared_state.lock().await;
            let now = std::time::Instant::now();

            // Check if this is a double ESC (within 2 seconds)
            if state.esc_pressed {
                if let Some(last_time) = state.last_esc_time {
                    if now.duration_since(last_time).as_secs() < 2 {
                        // Double ESC - disconnect
                        drop(state);
                        info!("Double ESC detected, disconnecting...");
                        if let Some(handle) = &self.session_handle {
                            let _ = handle.disconnect(
                                Disconnect::ByApplication,
                                "Disconnected by user".to_string(),
                                "en".to_string(),
                            ).await;
                        }
                        return Ok(());
                    }
                }
            }

            // First ESC or timeout - show hint
            state.esc_pressed = true;
            state.last_esc_time = Some(now);
            drop(state);

            // Send hint message
            let hint = terminal_ui::create_esc_hint();
            session.data(channel, hint.into_bytes().into())?;

            // Spawn task to clear hint after 2 seconds if no second ESC
            let shared_state = self.shared_state.clone();
            let handle = self.session_handle.clone();
            let channel_id = channel;
            tokio::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                let mut state = shared_state.lock().await;
                if state.esc_pressed {
                    state.esc_pressed = false;
                    state.last_esc_time = None;
                    // Clear the hint
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

        // Now send the activation message if Device Flow is pending
        let status = self.get_verification_status().await;
        if let VerificationStatus::Pending { code } = status {
            let url = self.device_flow_client.get_activation_url(&code);

            // Send activation box to client
            let message = terminal_ui::create_activation_box(&code, &url);
            if let Err(e) = session.data(channel, message.into_bytes().into()) {
                warn!("Failed to send activation message: {:?}", e);
            }
        }

        Ok(())
    }
}

impl SshHandler {
    /// Create tunnel after verification (used for SKIP_AUTH mode)
    async fn create_tunnel(&self, address: &str, port: u32) -> Result<bool, TunnelError> {
        let handle = match &self.session_handle {
            Some(h) => h.clone(),
            None => {
                error!("No session handle available!");
                return Ok(false);
            }
        };

        let subdomain = self.generate_subdomain().await;
        let username = {
            let state = self.shared_state.lock().await;
            match &state.verification_status {
                VerificationStatus::Verified { user_id } => user_id.clone(),
                _ => self.username.clone().unwrap_or_else(|| "anonymous".to_string()),
            }
        };

        let client_ip = self
            .peer_addr
            .map(|addr| addr.ip().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let tunnel_info = TunnelInfo {
            subdomain: subdomain.clone(),
            handle,
            requested_address: address.to_string(),
            requested_port: port,
            server_port: 80,
            created_at: Instant::now(),
            username,
            client_ip,
        };

        match self.state.register_tunnel(tunnel_info).await {
            Ok(()) => {
                let tunnel_url = get_tunnel_url(&subdomain);
                info!(
                    "✓ Tunnel registered!\n\
                     Subdomain: {}\n\
                     URL: {}",
                    subdomain, tunnel_url
                );
                self.shared_state.lock().await.registered_subdomains.push(subdomain);
                Ok(true)
            }
            Err(TunnelError::SubdomainTaken(s)) => {
                warn!("Subdomain {} already taken", s);
                Ok(false)
            }
            Err(e) => {
                error!("Failed to register tunnel: {}", e);
                Err(e)
            }
        }
    }
}
