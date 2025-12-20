import type React from 'react'

interface TerminalWindowProps {
  title: string
  children: React.ReactNode
}

export function TerminalWindow({ title, children }: TerminalWindowProps) {
  return (
    <div className="mx-auto max-w-2xl border border-border bg-background">
      {/* Title bar */}
      <div className="flex items-center justify-between border-border border-b px-4 py-2">
        <div className="flex items-center gap-2">
          <div className="flex gap-2">
            <span className="h-3 w-3 border border-destructive" />
            <span className="h-3 w-3 border border-secondary" />
            <span className="h-3 w-3 border border-primary" />
          </div>
          <span className="ml-4 text-muted-foreground text-xs">+--- {title} ---+</span>
        </div>
      </div>
      {/* Content */}
      <div className="p-4 font-mono text-sm sm:p-6">{children}</div>
    </div>
  )
}
