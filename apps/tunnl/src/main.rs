//! SSH Reverse Tunnel Server with HTTP Proxy
//!
//! ## Usage
//! ```bash
//! # Start the server
//! RUST_LOG=info cargo run
//!
//! # Connect SSH tunnel (in another terminal)
//! ssh -o StrictHostKeyChecking=no -N -R 80:localhost:3000 -p 2222 test@localhost
//!
//! # Access via HTTP proxy (server prints the subdomain)
//! curl -H "Host: tunnel-xxx.localhost" http://localhost:8080/
//! ```

use std::sync::Arc;

use log::info;
use russh::server::Server;

use tunnl::{load_or_generate_server_key, run_http_proxy, AppState, TunnelServer};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info")
    ).init();

    info!("🚀 Starting SSH Reverse Tunnel Server with HTTP Proxy...");

    let state = Arc::new(AppState::new());
    info!("✓ Application state initialized");

    let key = load_or_generate_server_key()?;

    let config = russh::server::Config {
        methods: russh::MethodSet::PUBLICKEY,
        server_id: russh::SshId::Standard("SSH-2.0-tunnl-0.1.0".to_string()),
        keys: vec![key],
        inactivity_timeout: Some(std::time::Duration::from_secs(1800)),
        auth_rejection_time: std::time::Duration::from_secs(3),
        auth_rejection_time_initial: Some(std::time::Duration::from_secs(0)),
        ..Default::default()
    };

    let config = Arc::new(config);
    let mut server = TunnelServer::new(state.clone());

    let ssh_addr = "0.0.0.0:2222";
    let http_addr = "0.0.0.0:8080";

    info!("═══════════════════════════════════════════════════════════════");
    info!("SSH server:   {}", ssh_addr);
    info!("HTTP proxy:   {}", http_addr);
    info!("═══════════════════════════════════════════════════════════════");
    info!("To create a tunnel:");
    info!("  ssh -o StrictHostKeyChecking=no -N -R 80:localhost:3000 -p 2222 test@localhost");
    info!("");
    info!("Then access via HTTP:");
    info!("  curl -H \"Host: tunnel-xxx.localhost\" http://localhost:8080/");
    info!("═══════════════════════════════════════════════════════════════");

    let http_state = state.clone();
    
    tokio::select! {
        result = server.run_on_address(config, ssh_addr) => {
            result?;
        }
        result = run_http_proxy(http_state, http_addr) => {
            result?;
        }
    }

    Ok(())
}
