'use client'

import { Check, Copy } from 'lucide-react'
import { useState } from 'react'
import { TerminalWindow } from './terminal-window'

const installCommands = [
  {
    title: 'Docker Compose',
    command: 'docker-compose up -d'
  },
  {
    title: 'Docker',
    command: 'docker pull ghcr.io/ZingerLittleBee/EXLO:latest'
  },
  {
    title: 'From Source',
    command: 'git clone https://github.com/ZingerLittleBee/EXLO && cd EXLO && cargo build --release'
  }
]

export function InstallationSection() {
  const [copied, setCopied] = useState<number | null>(null)

  const copyToClipboard = (text: string, index: number) => {
    navigator.clipboard.writeText(text).catch(() => {
      console.error('Copy failed')
    })
    setCopied(index)
    setTimeout(() => setCopied(null), 2000)
  }

  return (
    <section className="border-border border-t px-4 py-20 sm:px-6 lg:px-8" id="docs">
      <div className="mx-auto max-w-4xl">
        {/* Section header */}
        <div className="mb-16 text-center">
          <div className="mb-6 inline-block border border-border px-4 py-2">
            <span className="text-muted-foreground text-xs uppercase tracking-widest">$ sudo ./install.sh</span>
          </div>
          <h2 className="terminal-glow font-bold text-2xl uppercase tracking-tight sm:text-3xl">
            Self-Host in Minutes
          </h2>
          <p className="mx-auto mt-4 max-w-xl text-muted-foreground text-sm">
            Deploy your own tunnel server with Docker. Users connect with standard SSH — no client software required.
          </p>
          <div className="mx-auto mt-6 h-px w-32 bg-border" />
        </div>

        {/* Installation options */}
        <div className="space-y-6">
          {installCommands.map((item, index) => (
            <div className="border border-border" key={index}>
              {/* Title bar */}
              <div className="flex items-center justify-between border-border border-b px-4 py-2">
                <span className="text-muted-foreground text-xs uppercase tracking-widest">{item.title}</span>
                <button
                  aria-label="Copy command"
                  className="p-1 text-muted-foreground transition-colors hover:text-foreground"
                  onClick={() => copyToClipboard(item.command, index)}
                >
                  {copied === index ? <Check className="h-4 w-4 text-primary" /> : <Copy className="h-4 w-4" />}
                </button>
              </div>
              {/* Command */}
              <div className="overflow-x-auto p-4">
                <code className="terminal-glow text-foreground text-sm">$ {item.command}</code>
              </div>
            </div>
          ))}
        </div>

        {/* Environment config example */}
        <div className="mt-12">
          <TerminalWindow title=".env.example">
            <pre className="overflow-x-auto text-muted-foreground text-xs sm:text-sm">
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
        <div className="mt-12 text-center">
          <a
            className="inline-flex items-center gap-2 text-muted-foreground text-sm transition-colors hover:text-foreground"
            href="https://github.com/ZingerLittleBee/EXLO/wiki"
            rel="noopener noreferrer"
            target="_blank"
          >
            <span>{'>'}</span>
            <span className="uppercase tracking-wider">View Full Documentation</span>
            <span>{'--help'}</span>
          </a>
        </div>
      </div>
    </section>
  )
}
