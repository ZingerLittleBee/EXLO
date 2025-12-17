//! SSH handler for individual connections.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use log::{debug, error, info, warn};
use russh::keys::PublicKey;
use russh::server::{Auth, Handle, Handler, Msg, Session};
use russh::{Channel, ChannelId};
use russh_keys::HashAlg;

use crate::error::TunnelError;
use crate::state::{AppState, TunnelInfo};

/// Handler for a single SSH connection.
pub struct SshHandler {
    state: Arc<AppState>,
    peer_addr: Option<SocketAddr>,
    username: Option<String>,
    session_handle: Option<Handle>,
    registered_subdomains: Vec<String>,
    subdomain_counter: u32,
    /// Track the main session channel ID to only cleanup on session close
    session_channel_id: Option<ChannelId>,
}

impl SshHandler {
    pub fn new(state: Arc<AppState>, peer_addr: Option<SocketAddr>) -> Self {
        Self {
            state,
            peer_addr,
            username: None,
            session_handle: None,
            registered_subdomains: Vec::new(),
            subdomain_counter: 0,
            session_channel_id: None,
        }
    }

    fn generate_subdomain(&mut self) -> String {
        self.subdomain_counter += 1;
        let random_part: u32 = rand_simple();
        format!("tunnel-{:06x}-{}", random_part, self.subdomain_counter)
    }

    /// Cleanup all tunnels registered by this connection
    async fn cleanup_tunnels(&mut self) {
        for subdomain in &self.registered_subdomains {
            match self.state.remove_tunnel(subdomain).await {
                Ok(_) => info!("Removed tunnel: {}", subdomain),
                Err(e) => warn!("Failed to remove tunnel {}: {}", subdomain, e),
            }
        }
        self.registered_subdomains.clear();
    }
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
        // Only cleanup tunnels when the MAIN session channel closes
        // Not when forwarded TCP channels close (those are per-request)
        if self.session_channel_id == Some(channel) {
            info!("Session channel {:?} closed, cleaning up tunnels...", channel);
            self.cleanup_tunnels().await;
        } else {
            debug!("Forwarded channel {:?} closed (not cleaning up tunnels)", channel);
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
            "=== Virtual Tunnel Request ===\n\
             Address: '{}'\n\
             Port: {}\n\
             User: {:?}\n\
             Peer: {:?}",
            address, port, self.username, self.peer_addr
        );

        let handle = match &self.session_handle {
            Some(h) => h.clone(),
            None => {
                error!("No session handle available!");
                return Ok(false);
            }
        };

        let subdomain = self.generate_subdomain();
        let username = self.username.clone().unwrap_or_else(|| "anonymous".to_string());

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
                    "✓ Virtual tunnel registered!\n\
                     Subdomain: {}\n\
                     Access URL: http://{}.localhost:8080",
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
        _session: &mut Session,
    ) -> Result<bool, Self::Error> {
        let channel_id = channel.id();
        info!("Session channel opened: id={:?}", channel_id);
        // Store this as the main session channel
        self.session_channel_id = Some(channel_id);
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
