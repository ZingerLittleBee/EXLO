# tunnel

A high-performance SSH reverse tunnel server written in Rust.

## Features

- **SSH Server** (`:2222`) — Accepts `ssh -R` reverse port forwarding
- **HTTP Proxy** (`:8080`) — Subdomain-based TCP passthrough routing
- **Management API** (`:9090`) — Internal REST API for tunnel administration
- **Device Flow Auth** — Browser-based authentication, no SSH keys required
- **Virtual Bind** — No physical port binding, scales to thousands of tunnels
- **Reconnection Support** — Preserves subdomain within 30-minute window

## Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                           tunnel Server                               │
│  ┌──────────────┐   ┌──────────────┐   ┌──────────────────────┐     │
│  │  HTTP Proxy  │   │  SSH Server  │   │  Management API      │     │
│  │  :8080       │   │  :2222       │   │  :9090 (internal)    │     │
│  └──────┬───────┘   └──────┬───────┘   └──────────────────────┘     │
│         │                  │                                         │
│         └────────┬─────────┘                                         │
│                  ▼                                                   │
│            ┌──────────┐                                              │
│            │ AppState │  tunnels / verified_keys / rate_limits       │
│            └──────────┘                                              │
└─────────────────────────────────────────────────────────────────────┘
                │                              │
                ▼                              ▼
          curl client               ssh -R 8000:localhost:8000 -p 2222
                                               │
                                               ▼
                                      User's localhost:8000
```

## Project Structure

```
src/
├── main.rs          # Entry point, server initialization
├── lib.rs           # Public API exports
├── config.rs        # Environment configuration
├── state.rs         # AppState, TunnelInfo, VerifiedKey, RateLimiting
├── error.rs         # TunnelError enum
├── key.rs           # SSH server key persistence
├── proxy.rs         # TCP passthrough proxy with Host header peek
├── device.rs        # Device Flow client, activation code generation
├── management.rs    # REST API (axum) for tunnel management
├── terminal_ui.rs   # Terminal output formatting
└── ssh/
    ├── mod.rs          # Module exports
    ├── server.rs       # TunnelServer (russh Server impl)
    ├── handler.rs      # SshHandler struct and core methods
    ├── handler_impl.rs # Handler trait implementation (SSH callbacks)
    ├── tunnel.rs       # Tunnel creation logic
    ├── types.rs        # Shared types (PendingTunnel, VerificationStatus)
    └── verification.rs # Device Flow polling
```

## Tech Stack

| Component | Library |
|-----------|---------|
| SSH Protocol | `russh` / `russh-keys` |
| TCP Proxy | `tokio` (TcpStream, peek-based routing) |
| Management API | `axum` / `tower-http` |
| Async Runtime | `tokio` |
| HTTP Client | `reqwest` |
| Terminal UI | `console` |
| Error Handling | `thiserror` / `anyhow` |

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `SSH_PORT` | `2222` | SSH server port |
| `HTTP_PORT` | `8080` | HTTP proxy port |
| `MGMT_PORT` | `9090` | Management API port |
| `API_BASE_URL` | `http://localhost:3000` | Web app URL for Device Flow |
| `INTERNAL_API_SECRET` | `dev-secret` | Secret for internal API auth |
| `TUNNEL_URL` | `localhost` | Domain for tunnel subdomains |
| `RUST_LOG` | `info` | Log level |

## Usage

```bash
# Start the server
RUST_LOG=info cargo run

# Create tunnel (in another terminal)
ssh -R 8000:localhost:8000 -p 2222 user@localhost

# You'll see an activation URL — visit it in your browser
# After authorization, access via:
curl -H "Host: tunnel-xxx.localhost" http://localhost:8080/
```

## Disconnecting SSH

Press the following keys in sequence: `Enter` → `~` → `.`

## Management API

```bash
# List all tunnels
curl http://localhost:9090/tunnels

# Delete a tunnel
curl -X DELETE http://localhost:9090/tunnels/{subdomain}
```

## Data Flow

```
1. SSH Client connects       → ssh/server.rs (TunnelServer)
2. Device Flow auth          → ssh/verification.rs → Web API (/api/device-code)
3. Auth success              → state.rs registers tunnel (tunnels HashMap)
4. HTTP request arrives      → proxy.rs parses Host header → lookup tunnel → forward
5. SSH channel opens         → bidirectional TCP copy between client and tunnel
```

## Key Design Decisions

| Decision | Rationale |
|----------|-----------|
| **Virtual Bind** | No physical port binding per tunnel; uses subdomain routing to scale to thousands |
| **Device Flow Auth** | Browser-based OAuth flow instead of SSH keys for better UX and security |
| **Reconnection Window** | 30-minute grace period preserves subdomain on network interruptions |
| **Peek-based Routing** | Reads Host header without consuming bytes, enabling transparent TCP passthrough |
| **Sidecar Pattern** | Rust handles data plane (performance), Node.js handles control plane (auth, UI) |
