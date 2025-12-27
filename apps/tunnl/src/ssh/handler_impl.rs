//! Handler trait implementation for SshHandler.

use async_trait::async_trait;
use log::{debug, info, warn};
use russh::keys::PublicKey;
use russh::server::{Auth, Handler, Msg, Session};
use russh::{Channel, ChannelId, Disconnect};
use russh_keys::HashAlg;

use crate::config::is_development;
use crate::error::TunnelError;
use crate::terminal_ui;

use super::handler::SshHandler;
use super::types::{PendingTunnel, VerificationStatus};

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
                "Public key already verified for user '{}', subdomains={:?}, skipping Device Flow",
                verified_key.user_id, verified_key.subdomains
            );
            let display_name = verified_key.get_display_name();
            let mut state = self.shared_state.lock().await;
            state.verification_status = VerificationStatus::Verified {
                user_id: verified_key.user_id,
                display_name,
            };
            state.last_subdomains = verified_key.subdomains;
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
                let dev_user = self.username.clone().unwrap_or_else(|| "dev".to_string());
                state.verification_status = VerificationStatus::Verified {
                    user_id: dev_user.clone(),
                    display_name: dev_user,
                };
            }
            let result = self.do_create_tunnel(address, *port).await?;
            return Ok(result.success);
        }

        // If already verified (reconnection or new port), create tunnel immediately
        if self.is_verified().await {
            let result = self.do_create_tunnel(address, *port).await?;
            if result.success {
                self.send_tunnel_message(*port).await;
            }
            return Ok(result.success);
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
                Ok(_code) => {
                    debug!("Device Flow started for pending tunnel");
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

        // Check verification status for new connections
        let status = self.get_verification_status().await;

        match status {
            VerificationStatus::Verified { .. } => {
                // Already verified, tunnels will be created in tcpip_forward
            }
            VerificationStatus::NotStarted => {
                match self.start_device_flow().await {
                    Ok(code) => {
                        let url = self.device_flow_client.get_activation_url(&code);
                        debug!("Device Flow started - URL: {}", url);

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

        // Check if there's a pending tunnel message
        let pending_port = {
            let mut state = self.shared_state.lock().await;
            state.pending_tunnel_port.take()
        };

        if let Some(port) = pending_port {
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

            if !tunnels.is_empty() {
                let message = terminal_ui::create_success_box(&display_name, &tunnels);
                if let Err(e) = session.data(channel, message.into_bytes().into()) {
                    warn!("Failed to send tunnel message in shell_request: {:?}", e);
                } else {
                    info!("Tunnel message sent in shell_request");
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
