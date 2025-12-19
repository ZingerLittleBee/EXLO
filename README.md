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
- [ ] `user` / `session` tables (Better Auth).
- [ ] `invitations` table (Invite-only flow).
- [ ] `activation_codes` table (Device flow).

### Phase 2: Web Control Plane (Auth & Admin)
**Goal**: Secure the access points.
- [ ] **First Run Experience**: Redirect to `/setup` if no users exist.
- [ ] **Invite System**: Admin dashboard for generating `/join` links. Close public registration.

### Phase 3: Rust Core - SSH Server & Key Persistence
**Goal**: Stable, persistent SSH service.
- [ ] **Key Persistence**: Implementing `load_or_generate` logic for `id_ed25519` host key.
- [ ] **Virtual Bind**: Mapping `ssh -R` requests to internal channels without binding host ports.

### Phase 4: Sidecar Management API
**Goal**: Enable Data Plane <-> Control Plane communication.
- [ ] `axum` server on `:9090`.
- [ ] `GET /tunnels`: List active sessions.
- [ ] `DELETE /tunnels/:subdomain`: Terminate specific connections.

### Phase 5: Dashboard & Real-Time Monitor
**Goal**: "God Mode" UI for administration.
- [ ] BFF Loader for fetching tunnel status.
- [ ] Server Action `kickTunnel(subdomain)`.
- [ ] Real-time polling UI.

### Phase 6: Device Flow Integration
**Goal**: Seamless authentication for headless clients.
- [ ] SSH connection initiates Device Flow (generates code).
- [ ] Web `/activate` page for user verification.
- [ ] SSH session polling for verification status.