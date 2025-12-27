//! SSH Reverse Tunnel Server library.
//!
//! Provides components for building a tunnel service.

pub mod config;
pub mod device;
pub mod error;
pub mod key;
pub mod management;
pub mod proxy;
pub mod ssh;
pub mod state;
pub mod terminal_ui;

pub use config::{get_proxy_url, get_tunnel_url, is_development, validate_config};
pub use device::{generate_activation_code, truncate_user_id, DeviceFlowClient, DeviceFlowConfig, VerifiedUser};
pub use error::TunnelError;
pub use key::load_or_generate_server_key;
pub use management::run_management_api;
pub use proxy::run_http_proxy;
pub use ssh::{SshHandler, TunnelServer};
pub use state::{AppState, TunnelInfo, VerifiedKey};
