//! SSH server implementation.

use std::net::SocketAddr;
use std::sync::Arc;

use log::{error, info};
use russh::server::{Handler, Server};

use super::SshHandler;
use crate::device::DeviceFlowClient;
use crate::state::AppState;

/// The main SSH server that creates handlers for each connection.
#[derive(Clone)]
pub struct TunnelServer {
    state: Arc<AppState>,
    device_flow_client: Arc<DeviceFlowClient>,
}

impl TunnelServer {
    pub fn new(state: Arc<AppState>, device_flow_client: Arc<DeviceFlowClient>) -> Self {
        Self {
            state,
            device_flow_client,
        }
    }
}

impl Server for TunnelServer {
    type Handler = SshHandler;

    fn new_client(&mut self, peer_addr: Option<SocketAddr>) -> Self::Handler {
        info!("New SSH connection from {:?}", peer_addr);
        SshHandler::new(
            self.state.clone(),
            self.device_flow_client.clone(),
            peer_addr,
        )
    }

    fn handle_session_error(&mut self, error: <Self::Handler as Handler>::Error) {
        error!("Session error: {:?}", error);
    }
}
