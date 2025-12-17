//! Server key management.

use log::info;
use russh_keys::HashAlg;

/// Path to store the server key
const SERVER_KEY_PATH: &str = "server_key.pem";

/// Load server key from file, or generate a new one and save it.
pub fn load_or_generate_server_key() -> anyhow::Result<russh_keys::PrivateKey> {
    use russh_keys::Algorithm;
    use std::fs;
    use std::path::Path;

    let key_path = Path::new(SERVER_KEY_PATH);

    if key_path.exists() {
        info!("Loading server key from {}...", SERVER_KEY_PATH);
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

        let key_data = key.to_openssh(russh_keys::ssh_key::LineEnding::LF)?;
        fs::write(key_path, key_data.as_bytes())?;
        info!("Server key saved to {}", SERVER_KEY_PATH);
        info!(
            "Server key fingerprint: {}",
            key.public_key().fingerprint(HashAlg::Sha256)
        );

        Ok(key)
    }
}
