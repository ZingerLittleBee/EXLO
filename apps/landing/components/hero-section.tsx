"use client"

import { useEffect, useState } from "react"
import { TerminalWindow } from "./terminal-window"

const terminalLines = [
  { text: "$ ssh -p 2222 -R 80:localhost:3000 fwd.rs", delay: 0 },
  { text: "> Authenticating...", delay: 1500 },
  { text: "> Please visit https://fwd.rs/activate?code=XK9L", delay: 2500 },
  { text: "> Tunnel established: https://demo.fwd.rs", delay: 4000 },
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
    <section className="pt-32 pb-20 px-4 sm:px-6 lg:px-8">
      <div className="max-w-5xl mx-auto text-center">
        {/* Badge */}
        <div className="inline-flex items-center gap-2 px-3 py-1 border border-border mb-8">
          <span className="w-2 h-2 bg-primary animate-pulse" />
          <span className="text-xs uppercase tracking-widest text-muted-foreground">
            No Client Installation Required
          </span>
        </div>

        {/* Headline */}
        <h1 className="text-3xl sm:text-4xl lg:text-5xl font-bold uppercase tracking-tight terminal-glow mb-6 text-balance">
          Expose Localhost to the Internet. Securely.
        </h1>

        {/* Sub-headline */}
        <p className="text-muted-foreground text-sm sm:text-base max-w-2xl mx-auto mb-12 leading-relaxed">
          The self-hosted, clientless alternative to ngrok. Maintain full control over your data and traffic with strict
          access policies.
        </p>

        {/* Terminal Demo */}
        <TerminalWindow title="user@local:~">
          <div className="text-left space-y-2">
            {terminalLines.slice(0, visibleLines).map((line, index) => (
              <div
                key={index}
                className={`${line.text.includes("established") ? "text-primary terminal-glow" : line.text.startsWith(">") ? "text-secondary" : "text-foreground"}`}
              >
                {line.text}
              </div>
            ))}
            {visibleLines < terminalLines.length && <span className="inline-block w-3 h-5 bg-primary animate-blink" />}
            {visibleLines === terminalLines.length && (
              <div className="flex items-center gap-1 mt-4">
                <span className="text-foreground">$</span>
                <span className="inline-block w-3 h-5 bg-primary animate-blink" />
              </div>
            )}
          </div>
        </TerminalWindow>

        {/* CTA Buttons */}
        <div className="flex flex-col sm:flex-row items-center justify-center gap-4 mt-12">
          <a
            href="https://github.com/fwd-rs/fwd.rs"
            target="_blank"
            rel="noopener noreferrer"
            className="px-6 py-3 bg-primary text-primary-foreground font-bold uppercase tracking-wider hover:bg-primary/90 transition-colors text-sm"
          >
            [ VIEW ON GITHUB ]
          </a>
          <a
            href="#docs"
            className="px-6 py-3 border border-border text-foreground hover:border-primary hover:text-primary transition-colors uppercase tracking-wider text-sm"
          >
            [ READ THE DOCS ]
          </a>
        </div>
      </div>
    </section>
  )
}
