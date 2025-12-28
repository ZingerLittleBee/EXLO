# AGENTS.md

## Project Overview

A self-hosted SSH reverse tunnel service. Uses a **Sidecar architecture** with two main components communicating via internal API.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     Sidecar Pattern                          │
├──────────────────────────┬──────────────────────────────────┤
│   Data Plane (Rust)      │    Control Plane (Node.js)       │
│   apps/tunnl             │    apps/web                      │
├──────────────────────────┼──────────────────────────────────┤
│ • SSH Server (:2222)     │ • Web Dashboard (:3000)          │
│ • HTTP Proxy (:8080)     │ • Better Auth (SSR)              │
│ • Management API (:9090) │ • BFF → Rust Internal API        │
│ • russh + hyper + axum   │ • TanStack Start + Drizzle       │
└──────────────────────────┴──────────────────────────────────┘
                              │
                    ┌─────────┴─────────┐
                    │   PostgreSQL      │
                    │   (shared state)  │
                    └───────────────────┘
```

## Directory Structure

```
brisbane/
├── apps/
│   ├── tunnl/          # Rust SSH tunnel server
│   │   └── src/
│   │       ├── main.rs
│   │       ├── lib.rs
│   │       ├── ssh/           # SSH module
│   │       │   ├── server.rs      # TunnelServer
│   │       │   ├── handler.rs     # SshHandler struct
│   │       │   ├── handler_impl.rs # Handler trait impl
│   │       │   ├── tunnel.rs      # Tunnel creation
│   │       │   ├── types.rs       # Shared types
│   │       │   └── verification.rs # Device Flow polling
│   │       ├── proxy.rs       # HTTP proxy
│   │       ├── device.rs      # Device flow auth
│   │       ├── management.rs  # Internal API (:9090)
│   │       ├── state.rs       # AppState & TunnelInfo
│   │       └── terminal_ui.rs
│   ├── web/            # TanStack Start web dashboard
│   │   └── src/
│   │       ├── routes/     # File-based routing
│   │       ├── functions/  # Server functions
│   │       ├── components/
│   │       └── lib/
│   ├── landing/        # Landing page
│   └── docs/           # Documentation site
├── packages/
│   ├── db/             # Drizzle ORM + schemas
│   │   └── src/schema/
│   │       ├── auth.ts     # user, session, account
│   │       ├── tunnel.ts   # tunnels table
│   │       └── activation.ts # activation_codes
│   ├── auth/           # Better Auth configuration
│   └── config/         # Shared TypeScript config
├── docker/             # Dockerfiles
└── docker-compose*.yml # Deployment configs
```

## Tech Stack

| Layer | Technology |
|-------|------------|
| Data Plane | Rust, russh, hyper, axum, tokio |
| Control Plane | TanStack Start, React, Vite |
| Auth | Better Auth + PostgreSQL adapter |
| ORM | Drizzle ORM |
| Database | PostgreSQL |
| Package Manager | bun |
| Monorepo | Turborepo |
| Linting | Biome |
| Deployment | Docker, Traefik |

## Key Commands

```bash
# Install dependencies
bun install

# Start dev database
bun run db:start

# Push schema changes
bun run db:push

# Run web dashboard (dev)
bun run dev:web

# Run SSH tunnel server (dev)
cd apps/tunnl && RUST_LOG=info DATABASE_URL=postgresql://postgres:password@localhost:5432/exlo cargo run

# Docker deployment
docker compose up -d --build
```

## Ports

| Service | Port |
|---------|------|
| Web Dashboard | 3000 |
| SSH Server | 2222 |
| HTTP Proxy | 8080 |
| Management API (internal) | 9090 |

## Key Patterns

1. **Device Flow Auth**: SSH clients authenticate via web browser, not SSH keys
2. **Virtual Bind**: No physical port binding per tunnel, uses subdomain routing
3. **BFF Pattern**: Web app proxies requests to Rust internal API
4. **Invite-Only**: Public sign-up disabled, users join via invitation

## Database Schema

- `user`, `session`, `account`, `verification` - Better Auth tables
- `tunnels` - Active tunnel storage (subdomain, userId, ports)
- `activation_codes` - Device flow verification codes

## Environment Variables

```bash
# Required
DATABASE_URL=postgresql://...
AUTH_SECRET=<min-32-chars>
HOMEPAGE_URL=http://localhost:3000

# Optional
INTERNAL_API_SECRET=<for-rust-web-communication>
TUNNEL_URL=localhost:8080
```

## Development Flow

1. User connects via `ssh -R 8000:localhost:8000 -p 2222 user@host`
2. Rust server generates activation code, shows in terminal
3. User visits `/activate` in browser, enters code
4. On verification, tunnel is registered with user association
5. Traffic to `<subdomain>.domain:8080` routes to user's local service
