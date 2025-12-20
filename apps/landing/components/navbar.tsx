"use client"

import { useState } from "react"
import { Menu, X } from "lucide-react"
import Link from "next/link"

export function Navbar() {
  const [isOpen, setIsOpen] = useState(false)

  return (
    <nav className="fixed top-0 left-0 right-0 z-40 border-b border-border bg-background/95 backdrop-blur">
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
        <div className="flex items-center justify-between h-16">
          {/* Logo */}
          <Link href="/" className="flex items-center gap-2 terminal-glow hover-glitch">
            <span className="w-3 h-3 bg-primary animate-pulse" />
            <span className="text-lg font-bold tracking-tight text-foreground uppercase">fwd.rs</span>
          </Link>

          {/* Desktop Navigation */}
          <div className="hidden md:flex items-center gap-6">
            <Link
              href="#features"
              className="text-sm text-muted-foreground hover:text-foreground transition-colors uppercase tracking-wider"
            >
              Features
            </Link>
            <Link
              href="#docs"
              className="text-sm text-muted-foreground hover:text-foreground transition-colors uppercase tracking-wider"
            >
              Documentation
            </Link>
            <Link
              href="https://github.com/fwd-rs/fwd.rs"
              target="_blank"
              className="text-sm text-muted-foreground hover:text-foreground transition-colors uppercase tracking-wider"
            >
              GitHub
            </Link>
            <Link
              href="#console"
              className="px-4 py-2 border border-primary text-primary hover:bg-primary hover:text-primary-foreground transition-colors text-sm uppercase tracking-wider"
            >
              [ Console Login ]
            </Link>
          </div>

          {/* Mobile menu button */}
          <button onClick={() => setIsOpen(!isOpen)} className="md:hidden text-foreground p-2">
            {isOpen ? <X className="w-6 h-6" /> : <Menu className="w-6 h-6" />}
          </button>
        </div>

        {/* Mobile Navigation */}
        {isOpen && (
          <div className="md:hidden border-t border-border py-4 space-y-4">
            <Link
              href="#features"
              className="block text-sm text-muted-foreground hover:text-foreground uppercase tracking-wider"
              onClick={() => setIsOpen(false)}
            >
              {">"} Features
            </Link>
            <Link
              href="#docs"
              className="block text-sm text-muted-foreground hover:text-foreground uppercase tracking-wider"
              onClick={() => setIsOpen(false)}
            >
              {">"} Documentation
            </Link>
            <Link
              href="https://github.com/fwd-rs/fwd.rs"
              target="_blank"
              className="block text-sm text-muted-foreground hover:text-foreground uppercase tracking-wider"
            >
              {">"} GitHub
            </Link>
            <Link
              href="#console"
              className="block px-4 py-2 border border-primary text-primary hover:bg-primary hover:text-primary-foreground transition-colors text-sm uppercase tracking-wider w-fit"
            >
              [ Console Login ]
            </Link>
          </div>
        )}
      </div>
    </nav>
  )
}
