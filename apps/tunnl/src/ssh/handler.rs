//! SSH handler for individual connections with Device Flow authentication.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use log::{debug, error, info, warn};
use russh::keys::PublicKey;
use russh::server::{Auth, Handle, Handler, Msg, Session};
use russh::{Channel, ChannelId};
use russh_keys::HashAlg;
use tokio::sync::oneshot;

use crate::device::{DeviceFlowClient, DeviceFlowConfig, generate_activation_code};
use crate::error::TunnelError;
use crate::state::{AppState, TunnelInfo};

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
    registered_subdomains: Vec<String>,
    subdomain_counter: u32,
    session_channel_id: Option<ChannelId>,
    /// Device Flow verification status
    verification_status: VerificationStatus,
    /// Session ID for this connection
    session_id: String,
    /// Channel to cancel polling task
    poll_cancel: Option<oneshot::Sender<()>>,
}

impl SshHandler {
    pub fn new(
        state: Arc<AppState>,
        device_flow_client: Arc<DeviceFlowClient>,
        peer_addr: Option<SocketAddr>,
    ) -> Self {
        let session_id = generate_session_id();
        Self {
            state,
            device_flow_client,
            peer_addr,
            username: None,
            session_handle: None,
            registered_subdomains: Vec::new(),
            subdomain_counter: 0,
            session_channel_id: None,
            verification_status: VerificationStatus::NotStarted,
            session_id,
            poll_cancel: None,
        }
    }

    fn generate_subdomain(&mut self) -> String {
        self.subdomain_counter += 1;
        let random_part: u32 = rand_simple();
        format!("tunnel-{:06x}-{}", random_part, self.subdomain_counter)
    }

    async fn cleanup_tunnels(&mut self) {
        for subdomain in &self.registered_subdomains {
            match self.state.remove_tunnel(subdomain).await {
                Ok(_) => info!("Removed tunnel: {}", subdomain),
                Err(e) => warn!("Failed to remove tunnel {}: {}", subdomain, e),
            }
        }
        self.registered_subdomains.clear();
    }

    /// Check if this session is verified
    fn is_verified(&self) -> bool {
        matches!(self.verification_status, VerificationStatus::Verified { .. })
    }

    /// Start the Device Flow verification process
    async fn start_device_flow(&mut self, session: &mut Session) {
        let code = generate_activation_code();
        let session_id = self.session_id.clone();
        let client = self.device_flow_client.clone();

        // Register the code with the web server
        match client.register_code(&code, &session_id).await {
            Ok(()) => {
                let activation_url = client.get_activation_url(&code);
                
                // Send message to user
                let message = format!(
                    "\r\n\
                    ╔══════════════════════════════════════════════════════════════╗\r\n\
                    ║                    DEVICE ACTIVATION                         ║\r\n\
                    ╠══════════════════════════════════════════════════════════════╣\r\n\
                    ║                                                              ║\r\n\
                    ║  Your activation code: {:<10}                          ║\r\n\
                    ║                                                              ║\r\n\
                    ║  Please visit:                                               ║\r\n\
                    ║  {}  ║\r\n\
                    ║                                                              ║\r\n\
                    ║  Waiting for authorization...                                ║\r\n\
                    ╚══════════════════════════════════════════════════════════════╝\r\n\
                    \r\n",
                    code,
                    format!("{:<47}", activation_url)
                );

                // Send to the session channel
                if let Some(channel_id) = self.session_channel_id {
                    if let Err(e) = session.data(channel_id, message.into_bytes().into()) {
                        error!("Failed to send activation message: {:?}", e);
                    }
                }

                self.verification_status = VerificationStatus::Pending { code: code.clone() };

                // Start polling in background
                let (cancel_tx, cancel_rx) = oneshot::channel();
                self.poll_cancel = Some(cancel_tx);

                let poll_code = code.clone();
                let poll_client = client.clone();
                
                // We need a way to communicate back - store in a shared state
                // For now, we'll poll synchronously in tcpip_forward
                info!("Device Flow started with code: {}", code);
            }
            Err(e) => {
                error!("Failed to register activation code: {:?}", e);
                self.verification_status = VerificationStatus::Failed {
                    reason: e.to_string(),
                };
            }
        }
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
        self.session_handle = Some(session.handle());
        Ok(())
    }

    async fn channel_close(
        &mut self,
        channel: ChannelId,
        _session: &mut Session,
    ) -> Result<(), Self::Error> {
        if self.session_channel_id == Some(channel) {
            info!("Session channel {:?} closed, cleaning up...", channel);
            
            // Cancel any pending poll
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
        // Accept the connection but require Device Flow verification for tunnels
        Ok(Auth::Accept)
    }

    async fn tcpip_forward(
        &mut self,
        address: &str,
        port: &mut u32,
        session: &mut Session,
    ) -> Result<bool, Self::Error> {
        info!(
            "=== Tunnel Request ===\n\
             Address: '{}'\n\
             Port: {}\n\
             User: {:?}\n\
             Verified: {}",
            address, port, self.username, self.is_verified()
        );

        // Check if already verified
        if !self.is_verified() {
            // Start Device Flow if not already started
            if matches!(self.verification_status, VerificationStatus::NotStarted) {
                self.start_device_flow(session).await;
            }

            // Poll for verification (blocking for this request)
            if let VerificationStatus::Pending { code } = &self.verification_status.clone() {
                info!("Waiting for Device Flow verification...");
                
                match self.device_flow_client.poll_until_verified(code).await {
                    Ok(user_id) => {
                        info!("Device Flow verified! User: {}", user_id);
                        self.verification_status = VerificationStatus::Verified { user_id };
                        
                        // Notify user
                        if let Some(channel_id) = self.session_channel_id {
                            let msg = "\r\n✓ Authorized! Creating tunnel...\r\n\r\n";
                            let _ = session.data(channel_id, msg.as_bytes().to_vec().into());
                        }
                    }
                    Err(e) => {
                        warn!("Device Flow failed: {:?}", e);
                        self.verification_status = VerificationStatus::Failed {
                            reason: e.to_string(),
                        };
                        
                        if let Some(channel_id) = self.session_channel_id {
                            let msg = format!("\r\n✗ Authorization failed: {}\r\n", e);
                            let _ = session.data(channel_id, msg.into_bytes().into());
                        }
                        
                        return Ok(false);
                    }
                }
            }
        }

        // Now proceed with tunnel creation
        let handle = match &self.session_handle {
            Some(h) => h.clone(),
            None => {
                error!("No session handle available!");
                return Ok(false);
            }
        };

        let subdomain = self.generate_subdomain();
        let username = match &self.verification_status {
            VerificationStatus::Verified { user_id } => user_id.clone(),
            _ => self.username.clone().unwrap_or_else(|| "anonymous".to_string()),
        };

        let tunnel_info = TunnelInfo {
            subdomain: subdomain.clone(),
            handle,
            requested_address: address.to_string(),
            requested_port: *port,
            server_port: 80,
            created_at: Instant::now(),
            username,
        };

        match self.state.register_tunnel(tunnel_info).await {
            Ok(()) => {
                info!(
                    "✓ Tunnel registered!\n\
                     Subdomain: {}\n\
                     URL: http://{}.localhost:8080",
                    subdomain, subdomain
                );
                
                // Send URL to user
                if let Some(channel_id) = self.session_channel_id {
                    let msg = format!(
                        "\r\n✓ Tunnel active: http://{}.localhost:8080\r\n\r\n",
                        subdomain
                    );
                    let _ = session.data(channel_id, msg.into_bytes().into());
                }
                
                self.registered_subdomains.push(subdomain);
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

    async fn cancel_tcpip_forward(
        &mut self,
        address: &str,
        port: u32,
        _session: &mut Session,
    ) -> Result<bool, Self::Error> {
        info!("Cancel tcpip_forward: address='{}', port={}", address, port);

        let tunnels_to_remove: Vec<String> = self.registered_subdomains.clone();
        
        for subdomain in tunnels_to_remove {
            if let Ok(info) = self.state.remove_tunnel(&subdomain).await {
                if info.requested_address == address && info.requested_port == port {
                    self.registered_subdomains.retain(|s| s != &subdomain);
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
        
        // Start Device Flow when session opens
        self.start_device_flow(session).await;
        
        Ok(true)
    }

    async fn data(
        &mut self,
        channel: ChannelId,
        data: &[u8],
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        debug!("Data received on channel {:?}: {} bytes", channel, data.len());
        session.data(channel, data.to_vec().into())?;
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
}
