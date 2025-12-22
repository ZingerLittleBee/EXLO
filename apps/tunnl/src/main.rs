//! SSH Reverse Tunnel Server with HTTP Proxy and Device Flow Authentication
//!
//! ## Usage
//! ```bash
//! # Start the server
//! RUST_LOG=info cargo run
//!
//! # Connect SSH tunnel (in another terminal)
//! ssh -o StrictHostKeyChecking=no -N -R 80:localhost:3000 -p 2222 test@localhost
//!
//! # You will see an activation URL - visit it in your browser to authorize
//! # After authorization, access via HTTP proxy
//! curl -H "Host: tunnel-xxx.localhost" http://localhost:8080/
//! ```

use std::sync::Arc;

use log::info;
use russh::server::Server;

use tunnl::{
    load_or_generate_server_key, run_http_proxy, run_management_api, AppState, DeviceFlowClient,
    DeviceFlowConfig, TunnelServer,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    info!("🚀 Starting SSH Reverse Tunnel Server with Device Flow...");

    // Initialize shared state
    let state = Arc::new(AppState::new());
    info!("✓ Application state initialized");

    // Initialize Device Flow client
    let device_flow_config = DeviceFlowConfig::default();
    info!("✓ Device Flow API: {}", device_flow_config.api_base_url);
    let device_flow_client = Arc::new(DeviceFlowClient::new(device_flow_config));

    // Load or generate SSH server key
    let key = load_or_generate_server_key()?;

    // Configure SSH server
    let config = russh::server::Config {
        methods: russh::MethodSet::PUBLICKEY,
        server_id: russh::SshId::Standard(format!("SSH-2.0-EXLO-{}", env!("CARGO_PKG_VERSION"))),
        keys: vec![key],
        inactivity_timeout: Some(std::time::Duration::from_secs(1800)),
        auth_rejection_time: std::time::Duration::from_secs(3),
        auth_rejection_time_initial: Some(std::time::Duration::from_secs(0)),
        ..Default::default()
    };

    let config = Arc::new(config);
    let mut server = TunnelServer::new(state.clone(), device_flow_client);

    let ssh_port = std::env::var("SSH_PORT").unwrap_or_else(|_| "2222".to_string());
    let ssh_addr = format!("0.0.0.0:{}", ssh_port);
    let http_port = std::env::var("HTTP_PORT").unwrap_or_else(|_| "8080".to_string());
    let http_addr = format!("0.0.0.0:{}", http_port);
    let mgmt_addr = "0.0.0.0:9090";

    info!("═══════════════════════════════════════════════════════════════");
    info!("SSH server:     {}", ssh_addr);
    info!("HTTP proxy:     {}", http_addr);
    info!("Inner Management API: {}", mgmt_addr);
    info!("═══════════════════════════════════════════════════════════════");
    info!("To create a tunnel:");
    info!("  ssh -N -R 3000:localhost:3000 -p {} user@yourserver.com", ssh_port);
    info!("");
    info!("You will see an activation URL - visit it to authorize.");
    info!("═══════════════════════════════════════════════════════════════");

    let http_state = state.clone();
    let mgmt_state = state.clone();

    tokio::select! {
        result = server.run_on_address(config, ssh_addr) => {
            result?;
        }
        result = run_http_proxy(http_state, http_addr) => {
            result?;
        }
        result = run_management_api(mgmt_state, mgmt_addr) => {
            result?;
        }
    }

    Ok(())
}
