'use client'

import { useEffect, useState } from 'react'
import { TerminalWindow } from './terminal-window'

const terminalLines = [
  { text: '$ ssh -p 2222 -R 80:localhost:3000 your.domain', delay: 0 },
  { text: '> Authenticating...', delay: 1500 },
  { text: '> Please visit https://your.domain/activate?code=XK9L', delay: 2500 },
  { text: '> Tunnel established: https://xxx.your.domain', delay: 4000 }
]

export function HeroSection() {
  const [visibleLines, setVisibleLines] = useState<number>(0)

  useEffect(() => {
    const timers: NodeJS.Timeout[] = []

    terminalLines.forEach((line, index) => {
      const timer = setTimeout(() => {
        setVisibleLines(index + 1)
      }, line.delay)
      timers.push(timer)
    })

    return () => timers.forEach(clearTimeout)
  }, [])

  return (
    <section className="px-4 pt-32 pb-20 sm:px-6 lg:px-8">
      <div className="mx-auto max-w-5xl text-center">
        {/* Badge */}
        <div className="mb-8 inline-flex items-center gap-2 border border-border px-3 py-1">
          <span className="h-2 w-2 animate-pulse bg-primary" />
          <span className="text-muted-foreground text-xs uppercase tracking-widest">
            No Client Installation Required
          </span>
        </div>

        {/* Headline */}
        <h1 className="terminal-glow mb-6 text-balance font-bold text-3xl uppercase tracking-tight sm:text-4xl lg:text-5xl">
          Expose Localhost to the Internet. Securely.
        </h1>

        {/* Sub-headline */}
        <p className="mx-auto mb-12 max-w-2xl text-muted-foreground text-sm leading-relaxed sm:text-base">
          The self-hosted solution for secure reverse tunneling that works with your existing SSH. No client
          installation, no extra software — just one command.
        </p>

        {/* Terminal Demo */}
        <TerminalWindow title="user@local:~">
          <div className="space-y-2 text-left">
            {terminalLines.slice(0, visibleLines).map((line, index) => (
              <div
                className={`${line.text.includes('established') ? 'terminal-glow text-primary' : line.text.startsWith('>') ? 'text-secondary' : 'text-foreground'}`}
                key={index}
              >
                {line.text}
              </div>
            ))}
            {visibleLines < terminalLines.length && <span className="inline-block h-5 w-3 animate-blink bg-primary" />}
            {visibleLines === terminalLines.length && (
              <div className="mt-4 flex items-center gap-1">
                <span className="text-foreground">$</span>
                <span className="inline-block h-5 w-3 animate-blink bg-primary" />
              </div>
            )}
          </div>
        </TerminalWindow>

        {/* CTA Buttons */}
        <div className="mt-12 flex flex-col items-center justify-center gap-4 sm:flex-row">
          <a
            className="bg-primary px-6 py-3 font-bold text-primary-foreground text-sm uppercase tracking-wider transition-colors hover:bg-primary/90"
            href="https://github.com/ZingerLittleBee/EXLO"
            rel="noopener noreferrer"
            target="_blank"
          >
            [ VIEW ON GITHUB ]
          </a>
          <a
            className="border border-border px-6 py-3 text-foreground text-sm uppercase tracking-wider transition-colors hover:border-primary hover:text-primary"
            href="#docs"
          >
            [ READ THE DOCS ]
          </a>
        </div>
      </div>
    </section>
  )
}
