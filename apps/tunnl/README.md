# 🚀 tunnl - SSH Reverse Tunnel Server

A high-performance SSH reverse tunnel server written in Rust.

## Features

- **SSH Server** (port 2222) - Accepts `ssh -R` reverse port forwarding requests
- **HTTP Proxy** (port 8080) - Routes traffic to tunnels based on subdomain
- **Virtual Bind** - No actual port binding, scales to thousands of tunnels
- **Persistent Server Key** - No host key warnings after first connection

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                         Server                               │
│   ┌──────────────┐              ┌──────────────┐            │
│   │ HTTP Proxy   │◄────────────►│ SSH Server   │            │
│   │ Port 8080    │   AppState   │ Port 2222    │            │
│   └──────┬───────┘              └──────┬───────┘            │
└──────────│──────────────────────────────│───────────────────┘
           │                              │
           ▼                              ▼
      curl client              ssh -R 8000:localhost:8000 -p 2222 user@localhost
                                          │
                                          ▼
                                 User's localhost:8000
```

## Usage

```bash
# Terminal 1: Start the server
RUST_LOG=info cargo run

# Terminal 2: Start your local app
python3 -m http.server 3000

# Terminal 3: Create SSH tunnel
ssh -o StrictHostKeyChecking=no -R 8000:localhost:8000 -p 2222 test@localhost

# Terminal 4: Access via HTTP (use subdomain from server logs)
curl -H "Host: tunnel-xxx.localhost" http://localhost:8080/
```

## Disconnecting SSH

Since `Ctrl+C` does not work in SSH reverse tunnel mode, you can use the following method to disconnect:

Press the following keys in sequence:

1. **Enter** (Newline, to ensure you are at the start of a line)
2. **`~`** (Tilde)
3. **`.`** (Dot)

## Project Structure

```
src/
├── main.rs        # Entry point
├── lib.rs         # Library exports
├── error.rs       # TunnelError enum
├── state.rs       # TunnelInfo, AppState
├── key.rs         # Server key persistence
├── proxy.rs       # HTTP proxy layer
└── ssh/
    ├── mod.rs     # Module exports
    ├── server.rs  # TunnelServer
    └── handler.rs # SshHandler
```

## Tech Stack

- **russh** - SSH protocol
- **hyper** - HTTP server
- **tokio** - Async runtime
- **thiserror/anyhow** - Error handling
