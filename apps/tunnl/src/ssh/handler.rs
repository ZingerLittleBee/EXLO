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

use crate::device::{DeviceFlowClient, generate_activation_code};
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

    fn is_verified(&self) -> bool {
        matches!(self.verification_status, VerificationStatus::Verified { .. })
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

                self.verification_status = VerificationStatus::Pending { code: code.clone() };

                // Start polling cancellation channel
                let (cancel_tx, _cancel_rx) = oneshot::channel();
                self.poll_cancel = Some(cancel_tx);

                Ok(code)
            }
            Err(e) => {
                let reason = format!("Failed to register code: {}", e);
                error!("{}", reason);
                self.verification_status = VerificationStatus::Failed {
                    reason: reason.clone(),
                };
                Err(reason)
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
        info!(
            "=== Tunnel Request ===\n\
             Address: '{}'\n\
             Port: {}\n\
             User: {:?}\n\
             Status: {:?}",
            address, port, self.username, self.verification_status
        );

        // Skip auth completely if TUNNL_SKIP_AUTH is set
        if std::env::var("TUNNL_SKIP_AUTH").is_ok() {
            if !self.is_verified() {
                warn!("TUNNL_SKIP_AUTH is set - bypassing Device Flow verification");
                self.verification_status = VerificationStatus::Verified { 
                    user_id: self.username.clone().unwrap_or_else(|| "dev".to_string())
                };
            }
            return self.create_tunnel(address, *port).await;
        }

        // If already verified, proceed directly
        if self.is_verified() {
            return self.create_tunnel(address, *port).await;
        }

        // Start Device Flow if not already started (but don't block waiting for it)
        if matches!(self.verification_status, VerificationStatus::NotStarted) {
            match self.start_device_flow().await {
                Ok(code) => {
                    let url = self.device_flow_client.get_activation_url(&code);
                    info!("Device Flow started - Code: {}, URL: {}", code, url);
                    // Continue to create tunnel - user will see message in shell_request
                }
                Err(reason) => {
                    warn!("Device Flow failed: {}", reason);
                    return Ok(false);
                }
            }
        }

        // Create tunnel immediately - verification will happen in background
        // The tunnel is "pending" until verified
        self.create_tunnel(address, *port).await
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

        // Start Device Flow if not already started or verified
        if matches!(self.verification_status, VerificationStatus::NotStarted) {
            match self.start_device_flow().await {
                Ok(code) => {
                    let url = self.device_flow_client.get_activation_url(&code);
                    
                    // Log to server
                    info!("Device Flow started - Code: {}, URL: {}", code, url);

                    // Send message to client
                    let message = format!(
                        "\r\n\
                        ╔══════════════════════════════════════════════════════════════╗\r\n\
                        ║                    DEVICE ACTIVATION                         ║\r\n\
                        ╠══════════════════════════════════════════════════════════════╣\r\n\
                        ║                                                              ║\r\n\
                        ║  Code: {:<10}                                           ║\r\n\
                        ║                                                              ║\r\n\
                        ║  Visit: {:<50} ║\r\n\
                        ║                                                              ║\r\n\
                        ║  Waiting for authorization...                                ║\r\n\
                        ╚══════════════════════════════════════════════════════════════╝\r\n\
                        \r\n",
                        code, url
                    );
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
        if let VerificationStatus::Pending { code } = &self.verification_status.clone() {
            let url = self.device_flow_client.get_activation_url(code);
            
            let message = format!(
                "\r\n\
                ╔══════════════════════════════════════════════════════════════╗\r\n\
                ║                    DEVICE ACTIVATION                         ║\r\n\
                ╠══════════════════════════════════════════════════════════════╣\r\n\
                ║                                                              ║\r\n\
                ║  Code: {:<10}                                           ║\r\n\
                ║                                                              ║\r\n\
                ║  Visit: {:<50} ║\r\n\
                ║                                                              ║\r\n\
                ║  Waiting for authorization...                                ║\r\n\
                ╚══════════════════════════════════════════════════════════════╝\r\n\
                \r\n",
                code, url
            );
            
            if let Err(e) = session.data(channel, message.into_bytes().into()) {
                warn!("Failed to send activation message: {:?}", e);
            }
        }

        Ok(())
    }
}

impl SshHandler {
    /// Create tunnel after verification
    async fn create_tunnel(&mut self, address: &str, port: u32) -> Result<bool, TunnelError> {
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
                info!(
                    "✓ Tunnel registered!\n\
                     Subdomain: {}\n\
                     URL: http://{}.localhost:8080",
                    subdomain, subdomain
                );
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
}
