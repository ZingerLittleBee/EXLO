//! Server key management.

use log::info;
use russh_keys::HashAlg;

/// Path to store the server key
pub fn load_or_generate_server_key() -> anyhow::Result<russh_keys::PrivateKey> {
    use russh_keys::Algorithm;
    use std::env;
    use std::fs;
    use std::path::Path;

    let key_path_str = env::var("SERVER_KEY_PATH").unwrap_or_else(|_| "server_key.pem".to_string());
    let key_path = Path::new(&key_path_str);

    if key_path.exists() {
        info!("Loading server key from {}...", key_path.display());
        let key_data = fs::read_to_string(key_path)?;
        let key = russh_keys::PrivateKey::from_openssh(&key_data)?;
        info!(
            "Server key fingerprint: {}",
            key.public_key().fingerprint(HashAlg::Sha256)
        );
        Ok(key)
    } else {
        info!("Generating new Ed25519 server key...");
        let key = russh_keys::PrivateKey::random(&mut rand::thread_rng(), Algorithm::Ed25519)?;

        if let Some(parent) = key_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let key_data = key.to_openssh(russh_keys::ssh_key::LineEnding::LF)?;
        fs::write(key_path, key_data.as_bytes())?;
        info!("Server key saved to {}", key_path.display());
        info!(
            "Server key fingerprint: {}",
            key.public_key().fingerprint(HashAlg::Sha256)
        );

        Ok(key)
    }
}
