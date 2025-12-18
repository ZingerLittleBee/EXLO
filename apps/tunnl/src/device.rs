//! Device Flow authentication for SSH connections.
//!
//! This module implements the "Device Flow" where SSH clients authenticate
//! via a web browser instead of SSH keys.

use std::time::Duration;

use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};

/// Configuration for the Device Flow
#[derive(Clone)]
pub struct DeviceFlowConfig {
    /// Base URL of the web API (e.g., "http://localhost:3000")
    pub api_base_url: String,
    /// Internal API secret for authentication
    pub internal_secret: String,
    /// How long codes are valid (in seconds)
    pub code_expiry_secs: u64,
    /// How often to poll for verification (in seconds)
    pub poll_interval_secs: u64,
    /// Maximum poll attempts before giving up
    pub max_poll_attempts: u32,
}

impl Default for DeviceFlowConfig {
    fn default() -> Self {
        Self {
            api_base_url: std::env::var("API_BASE_URL")
                .unwrap_or_else(|_| "http://localhost:3000".to_string()),
            internal_secret: std::env::var("INTERNAL_API_SECRET")
                .unwrap_or_else(|_| "dev-secret".to_string()),
            code_expiry_secs: 300, // 5 minutes
            poll_interval_secs: 2,
            max_poll_attempts: 150, // 5 minutes at 2 sec intervals
        }
    }
}

/// Request to generate a new activation code
#[derive(Debug, Serialize)]
pub struct GenerateCodeRequest {
    pub code: String,
    #[serde(rename = "sessionId")]
    pub session_id: String,
    #[serde(rename = "expiresAt")]
    pub expires_at: String,
}

/// Response from code generation
#[derive(Debug, Deserialize)]
pub struct GenerateCodeResponse {
    pub success: Option<bool>,
    pub error: Option<String>,
}

/// Response from checking code status
#[derive(Debug, Deserialize)]
pub struct CheckCodeResponse {
    pub status: String,
    #[serde(rename = "userId")]
    pub user_id: Option<String>,
    pub error: Option<String>,
}

/// Generate a random activation code (e.g., "AF3D-1234")
pub fn generate_activation_code() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let part1: u16 = rng.gen_range(0..0xFFFF);
    let part2: u16 = rng.gen_range(0..0xFFFF);
    format!("{:04X}-{:04X}", part1, part2)
}

/// Device Flow API client
pub struct DeviceFlowClient {
    config: DeviceFlowConfig,
    http_client: reqwest::Client,
}

impl DeviceFlowClient {
    pub fn new(config: DeviceFlowConfig) -> Self {
        Self {
            config,
            http_client: reqwest::Client::builder()
                .no_proxy()  // Bypass system proxy (e.g., Surge)
                .build()
                .expect("Failed to build HTTP client"),
        }
    }

    /// Register a new activation code with the web server
    pub async fn register_code(
        &self,
        code: &str,
        session_id: &str,
    ) -> Result<(), anyhow::Error> {
        let expires_at = chrono_lite::now_plus_secs(self.config.code_expiry_secs);
        
        let request = GenerateCodeRequest {
            code: code.to_string(),
            session_id: session_id.to_string(),
            expires_at,
        };

        let url = format!("{}/api/internal/generate-code", self.config.api_base_url);
        
        let response = self
            .http_client
            .post(&url)
            .header("X-Internal-Secret", &self.config.internal_secret)
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to register code: {} - {}", status, body);
        }

        let result: GenerateCodeResponse = response.json().await?;
        
        if let Some(error) = result.error {
            anyhow::bail!("API error: {}", error);
        }

        info!("Registered activation code: {}", code);
        Ok(())
    }

    /// Check if a code has been verified
    pub async fn check_code(&self, code: &str) -> Result<CheckCodeResponse, anyhow::Error> {
        let url = format!(
            "{}/api/internal/check-code?code={}",
            self.config.api_base_url, code
        );

        let response = self
            .http_client
            .get(&url)
            .header("X-Internal-Secret", &self.config.internal_secret)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to check code: {} - {}", status, body);
        }

        let result: CheckCodeResponse = response.json().await?;
        Ok(result)
    }

    /// Poll until the code is verified or times out
    pub async fn poll_until_verified(
        &self,
        code: &str,
    ) -> Result<String, anyhow::Error> {
        let interval = Duration::from_secs(self.config.poll_interval_secs);
        
        for attempt in 0..self.config.max_poll_attempts {
            tokio::time::sleep(interval).await;

            match self.check_code(code).await {
                Ok(response) => {
                    debug!("Poll attempt {}: status={}", attempt + 1, response.status);
                    
                    match response.status.as_str() {
                        "verified" => {
                            if let Some(user_id) = response.user_id {
                                info!("Code {} verified by user {}", code, user_id);
                                return Ok(user_id);
                            }
                        }
                        "expired" => {
                            anyhow::bail!("Activation code expired");
                        }
                        "not_found" => {
                            anyhow::bail!("Activation code not found");
                        }
                        "pending" => {
                            // Continue polling
                        }
                        other => {
                            warn!("Unknown status: {}", other);
                        }
                    }
                }
                Err(e) => {
                    warn!("Poll error (attempt {}): {}", attempt + 1, e);
                    // Continue polling on transient errors
                }
            }
        }

        anyhow::bail!("Timeout waiting for activation")
    }

    /// Get the activation URL for display to the user
    pub fn get_activation_url(&self, code: &str) -> String {
        format!("{}/activate?code={}", self.config.api_base_url, code)
    }
}

/// Simple time helper (no external chrono dependency)
mod chrono_lite {
    use std::time::{SystemTime, UNIX_EPOCH};

    pub fn now_plus_secs(secs: u64) -> String {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let future = now + secs;
        // Format as ISO 8601
        format_timestamp(future)
    }

    fn format_timestamp(unix_secs: u64) -> String {
        // Convert to rough ISO format (good enough for JS Date parsing)
        let days_since_epoch = unix_secs / 86400;
        let secs_in_day = unix_secs % 86400;
        
        // Simple calculation (not accounting for leap years accurately, but close enough)
        let mut year = 1970;
        let mut remaining_days = days_since_epoch;
        
        loop {
            let days_in_year = if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) {
                366
            } else {
                365
            };
            if remaining_days < days_in_year {
                break;
            }
            remaining_days -= days_in_year;
            year += 1;
        }
        
        let days_in_months = if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) {
            [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
        } else {
            [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
        };
        
        let mut month = 0;
        for (i, &days) in days_in_months.iter().enumerate() {
            if remaining_days < days as u64 {
                month = i + 1;
                break;
            }
            remaining_days -= days as u64;
        }
        
        let day = remaining_days + 1;
        let hours = secs_in_day / 3600;
        let minutes = (secs_in_day % 3600) / 60;
        let seconds = secs_in_day % 60;
        
        format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
            year, month, day, hours, minutes, seconds
        )
    }
}
