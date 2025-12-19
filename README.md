# Self-Hosted SSH Reverse Tunnel Service

A self-hosted, open-source alternative to ngrok/tunnl.gg, designed for privacy and control.

## Project Manifesto

1.  **Self-Hosted & Private**: Strictly for private deployment. No public sign-ups. Your infrastructure, your rules.
2.  **Clientless**: Connect transparently using standard `ssh -R`. No custom CLI installation required on client machines.
3.  **Secure**: Full administrative control via a Web Dashboard. Monitor active tunnels and terminate connections instantly.

## Tech Stack & Architecture

The system is composed of two primary containers operating in a "Sidecar" pattern.

### 1. Data Plane (Rust Container)
*   **Core**: Built on `russh` (SSH Server on `:2222`) and `hyper` (HTTP Proxy on `:8080`).
*   **State**: In-memory `Arc<RwLock<AppState>>` synchronized with PostgreSQL.
*   **Internal API**: `axum` server on `:9090` for management operations (**Internal Only**).
*   **Key Features**: "Virtual Bind" (no physical port binding) and persistent Host Keys.

### 2. Control Plane (Node.js Container)
*   **Framework**: TanStack Start (Server-Side Rendering).
*   **Auth**: Better Auth with PostgreSQL adapter.
*   **Database**: Drizzle ORM.
*   **Pattern**: Backend for Frontend (BFF). Proxies requests to the Rust Internal API.

## Development Roadmap

### Phase 1: Foundation & Schema (PostgreSQL/Drizzle)
**Goal**: Set up data structures for strict access control.
- [x] `user` / `session` tables (Better Auth).
- [ ] `invitations` table (Invite-only flow).
- [x] `activation_codes` table (Device flow).
- [x] `tunnels` table (Persistent tunnel storage).

### Phase 2: Web Control Plane (Auth & Admin)
**Goal**: Secure the access points.
- [ ] **First Run Experience**: Redirect to `/setup` if no users exist.
- [ ] **Invite System**: Admin dashboard for generating `/join` links. Close public registration.

### Phase 3: Rust Core - SSH Server & Key Persistence
**Goal**: Stable, persistent SSH service.
- [x] **Key Persistence**: Implementing `load_or_generate` logic for `id_ed25519` host key.
- [x] **Virtual Bind**: Mapping `ssh -R` requests to internal channels without binding host ports.
- [x] **Terminal UI**: Beautiful box-drawing UI using `console` crate for device activation.

### Phase 4: Sidecar Management API
**Goal**: Enable Data Plane <-> Control Plane communication.
- [x] `axum` server on `:9090`.
- [x] `GET /tunnels`: List active sessions.
- [x] `DELETE /tunnels/:subdomain`: Terminate specific connections.
- [x] Internal API endpoints for tunnel registration/unregistration.

### Phase 5: Dashboard & Real-Time Monitor
**Goal**: "God Mode" UI for administration.
- [ ] BFF Loader for fetching tunnel status.
- [ ] Server Action `kickTunnel(subdomain)`.
- [ ] Real-time polling UI.

### Phase 6: Device Flow Integration
**Goal**: Seamless authentication for headless clients.
- [x] SSH connection initiates Device Flow (generates code).
- [x] Web `/activate` page for user verification.
- [x] SSH session polling for verification status.
- [x] Loading spinner animation during authorization.
- [x] Success/error UI boxes with styled output.