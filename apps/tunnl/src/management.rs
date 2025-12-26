//! Management API for tunnel administration.
//!
//! Provides HTTP endpoints for listing and managing active tunnels.

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get},
    Json, Router,
};
use chrono::{DateTime, Utc};
use log::{error, info};
use serde::Serialize;
use tower_http::cors::{Any, CorsLayer};

use crate::state::AppState;

/// JSON response for a single tunnel.
#[derive(Debug, Serialize)]
pub struct TunnelResponse {
    pub subdomain: String,
    pub user_id: Option<String>,
    pub client_ip: String,
    pub connected_at: String,
    /// Whether the SSH connection is still active (not closed)
    pub is_connected: bool,
}

/// JSON response for list of tunnels.
#[derive(Debug, Serialize)]
pub struct TunnelsListResponse {
    pub tunnels: Vec<TunnelResponse>,
}

/// JSON response for successful operations.
#[derive(Debug, Serialize)]
pub struct SuccessResponse {
    pub success: bool,
    pub message: String,
}

/// JSON response for errors.
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

/// GET /tunnels - List all active tunnels
async fn list_tunnels(
    State(state): State<Arc<AppState>>,
) -> Json<TunnelsListResponse> {
    let tunnels = state.list_tunnels().await;

    let tunnel_responses: Vec<TunnelResponse> = tunnels
        .into_iter()
        .map(|t| {
            // Convert SystemTime to DateTime<Utc>
            let connected_at: DateTime<Utc> = t.created_at.into();

            TunnelResponse {
                subdomain: t.subdomain,
                user_id: if t.username.is_empty() || t.username == "anonymous" {
                    None
                } else {
                    Some(t.username)
                },
                client_ip: t.client_ip,
                connected_at: connected_at.to_rfc3339(),
                is_connected: t.is_connected,
            }
        })
        .collect();

    Json(TunnelsListResponse { tunnels: tunnel_responses })
}

/// DELETE /tunnels/:subdomain - Force disconnect a tunnel
async fn kick_tunnel(
    State(state): State<Arc<AppState>>,
    Path(subdomain): Path<String>,
) -> Result<Json<SuccessResponse>, (StatusCode, Json<ErrorResponse>)> {
    info!("Management API: Kick request for tunnel '{}'", subdomain);

    match state.remove_tunnel(&subdomain).await {
        Ok(tunnel_info) => {
            // Send disconnect to the SSH session
            // This will cause the SSH session handle to be dropped when not used
            // Any future requests to this tunnel will fail with "tunnel not found"
            let handle = tunnel_info.handle;

            // Spawn a task to disconnect the session without blocking
            tokio::spawn(async move {
                // disconnect() gracefully closes the SSH connection
                if let Err(e) = handle.disconnect(
                    russh::Disconnect::ByApplication,
                    "Tunnel terminated by administrator".to_string(),
                    "en".to_string(),
                ).await {
                    log::debug!("Disconnect result: {:?}", e);
                }
            });

            info!("Management API: Tunnel '{}' kicked successfully", subdomain);
            Ok(Json(SuccessResponse {
                success: true,
                message: format!("Tunnel '{}' disconnected", subdomain),
            }))
        }
        Err(e) => {
            error!("Management API: Failed to kick tunnel '{}': {}", subdomain, e);
            Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: format!("Tunnel not found: {}", subdomain),
                }),
            ))
        }
    }
}

/// Create the management API router
pub fn create_management_router(state: Arc<AppState>) -> Router {
    // CORS configuration - allow requests from the web frontend
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/tunnels", get(list_tunnels))
        .route("/tunnels/{subdomain}", delete(kick_tunnel))
        .layer(cors)
        .with_state(state)
}

/// Run the management API server
pub async fn run_management_api(state: Arc<AppState>, addr: &str) -> anyhow::Result<()> {
    let router = create_management_router(state);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!("Management API listening on {}", addr);

    axum::serve(listener, router).await?;

    Ok(())
}
