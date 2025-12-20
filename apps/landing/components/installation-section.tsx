"use client"

import { useState } from "react"
import { Copy, Check } from "lucide-react"
import { TerminalWindow } from "./terminal-window"

const installCommands = [
  {
    title: "Docker Compose",
    command: "docker-compose up -d",
  },
  {
    title: "Docker",
    command: "docker pull ghcr.io/ZingerLittleBee/EXLO:latest",
  },
  {
    title: "From Source",
    command: "git clone https://github.com/ZingerLittleBee/EXLO && cd EXLO && cargo build --release",
  },
]

export function InstallationSection() {
  const [copied, setCopied] = useState<number | null>(null)

  const copyToClipboard = (text: string, index: number) => {
    navigator.clipboard.writeText(text)
    setCopied(index)
    setTimeout(() => setCopied(null), 2000)
  }

  return (
    <section id="docs" className="py-20 px-4 sm:px-6 lg:px-8 border-t border-border">
      <div className="max-w-4xl mx-auto">
        {/* Section header */}
        <div className="text-center mb-16">
          <div className="inline-block border border-border px-4 py-2 mb-6">
            <span className="text-xs text-muted-foreground uppercase tracking-widest">$ sudo ./install.sh</span>
          </div>
          <h2 className="text-2xl sm:text-3xl font-bold uppercase tracking-tight terminal-glow">
            Self-Host in Minutes
          </h2>
          <p className="text-muted-foreground text-sm mt-4 max-w-xl mx-auto">
            Deploy your own tunnel server with Docker. Full control, zero dependencies on external services.
          </p>
          <div className="w-32 h-px bg-border mx-auto mt-6" />
        </div>

        {/* Installation options */}
        <div className="space-y-6">
          {installCommands.map((item, index) => (
            <div key={index} className="border border-border">
              {/* Title bar */}
              <div className="border-b border-border px-4 py-2 flex items-center justify-between">
                <span className="text-xs text-muted-foreground uppercase tracking-widest">{item.title}</span>
                <button
                  onClick={() => copyToClipboard(item.command, index)}
                  className="text-muted-foreground hover:text-foreground transition-colors p-1"
                  aria-label="Copy command"
                >
                  {copied === index ? <Check className="w-4 h-4 text-primary" /> : <Copy className="w-4 h-4" />}
                </button>
              </div>
              {/* Command */}
              <div className="p-4 overflow-x-auto">
                <code className="text-sm text-foreground terminal-glow">$ {item.command}</code>
              </div>
            </div>
          ))}
        </div>

        {/* Environment config example */}
        <div className="mt-12">
          <TerminalWindow title=".env.example">
            <pre className="text-xs sm:text-sm text-muted-foreground overflow-x-auto">
              {`# Server Configuration
DOMAIN=your.domain
SSH_PORT=2222
HTTP_PORT=80
HTTPS_PORT=443

# Authentication
AUTH_PROVIDER=github
GITHUB_CLIENT_ID=your_client_id
GITHUB_CLIENT_SECRET=your_client_secret

# Database
DATABASE_URL=postgres://user:pass@localhost/fwd

# TLS (auto-generated if not provided)
TLS_CERT_PATH=/etc/ssl/certs/fwd.crt
TLS_KEY_PATH=/etc/ssl/private/fwd.key`}
            </pre>
          </TerminalWindow>
        </div>

        {/* Documentation link */}
        <div className="text-center mt-12">
          <a
            href="https://github.com/ZingerLittleBee/EXLO/wiki"
            target="_blank"
            rel="noopener noreferrer"
            className="inline-flex items-center gap-2 text-sm text-muted-foreground hover:text-foreground transition-colors"
          >
            <span>{">"}</span>
            <span className="uppercase tracking-wider">View Full Documentation</span>
            <span>{"--help"}</span>
          </a>
        </div>
      </div>
    </section>
  )
}
