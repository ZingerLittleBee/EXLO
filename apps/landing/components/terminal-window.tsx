import type React from "react"
interface TerminalWindowProps {
  title: string
  children: React.ReactNode
}

export function TerminalWindow({ title, children }: TerminalWindowProps) {
  return (
    <div className="border border-border bg-background max-w-2xl mx-auto">
      {/* Title bar */}
      <div className="border-b border-border px-4 py-2 flex items-center justify-between">
        <div className="flex items-center gap-2">
          <div className="flex gap-2">
            <span className="w-3 h-3 border border-destructive" />
            <span className="w-3 h-3 border border-secondary" />
            <span className="w-3 h-3 border border-primary" />
          </div>
          <span className="text-xs text-muted-foreground ml-4">+--- {title} ---+</span>
        </div>
      </div>
      {/* Content */}
      <div className="p-4 sm:p-6 font-mono text-sm">{children}</div>
    </div>
  )
}
