'use client'

import { Menu, X } from 'lucide-react'
import Link from 'next/link'
import { useState } from 'react'

export function Navbar() {
  const [isOpen, setIsOpen] = useState(false)

  return (
    <nav className="fixed top-0 right-0 left-0 z-40 border-border border-b bg-background/95 backdrop-blur">
      <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
        <div className="flex h-16 items-center justify-between">
          {/* Logo */}
          <Link className="terminal-glow hover-glitch flex items-center gap-2" href="/">
            {/* <Image alt="logo" className="stroke-red-500" height={28} src="/logo.svg" width={28} /> */}

            <svg className="size-7" height="64" viewBox="0 0 16 16" width="64" xmlns="http://www.w3.org/2000/svg">
              <title>logo</title>
              <path
                clipRule="evenodd"
                d="M4.256 6.041a3.75 3.75 0 0 1 7.348-.832l.152.528l.55.014a2.25 2.25 0 0 1 1.069 4.198a.75.75 0 1 0 .75 1.299a3.75 3.75 0 0 0-1.25-6.946a5.251 5.251 0 0 0-10.035.974a3.25 3.25 0 0 0-.896 6.2a.75.75 0 1 0 .603-1.373A1.75 1.75 0 0 1 3.25 6.75h.967zM6.22 7.22a.75.75 0 0 1 1.06 0l1.75 1.75l.53.53l-.53.53l-1.75 1.75a.75.75 0 0 1-1.06-1.06L7.44 9.5L6.22 8.28a.75.75 0 0 1 0-1.06M8 13.25a.75.75 0 0 1 .75-.75h2.5a.75.75 0 0 1 0 1.5h-2.5a.75.75 0 0 1-.75-.75"
                fill="currentColor"
                fillRule="evenodd"
              />
            </svg>

            <span className="font-bold text-foreground text-lg uppercase tracking-tight">EXLO</span>
          </Link>

          {/* Desktop Navigation */}
          <div className="hidden items-center gap-6 md:flex">
            <Link
              className="text-muted-foreground text-sm uppercase tracking-wider transition-colors hover:text-foreground"
              href="#features"
            >
              Features
            </Link>
            <Link
              className="text-muted-foreground text-sm uppercase tracking-wider transition-colors hover:text-foreground"
              href="#docs"
            >
              Documentation
            </Link>
            <Link
              className="text-muted-foreground text-sm uppercase tracking-wider transition-colors hover:text-foreground"
              href="https://github.com/ZingerLittleBee/EXLO"
              target="_blank"
            >
              GitHub
            </Link>
            <Link
              className="border border-primary px-4 py-2 text-primary text-sm uppercase tracking-wider transition-colors hover:bg-primary hover:text-primary-foreground"
              href="#console"
            >
              [ Console Login ]
            </Link>
          </div>

          {/* Mobile menu button */}
          <button className="p-2 text-foreground md:hidden" onClick={() => setIsOpen(!isOpen)}>
            {isOpen ? <X className="h-6 w-6" /> : <Menu className="h-6 w-6" />}
          </button>
        </div>

        {/* Mobile Navigation */}
        {isOpen && (
          <div className="space-y-4 border-border border-t py-4 md:hidden">
            <Link
              className="block text-muted-foreground text-sm uppercase tracking-wider hover:text-foreground"
              href="#features"
              onClick={() => setIsOpen(false)}
            >
              {'>'} Features
            </Link>
            <Link
              className="block text-muted-foreground text-sm uppercase tracking-wider hover:text-foreground"
              href="#docs"
              onClick={() => setIsOpen(false)}
            >
              {'>'} Documentation
            </Link>
            <Link
              className="block text-muted-foreground text-sm uppercase tracking-wider hover:text-foreground"
              href="https://github.com/ZingerLittleBee/EXLO"
              target="_blank"
            >
              {'>'} GitHub
            </Link>
            <Link
              className="block w-fit border border-primary px-4 py-2 text-primary text-sm uppercase tracking-wider transition-colors hover:bg-primary hover:text-primary-foreground"
              href="#console"
            >
              [ Console Login ]
            </Link>
          </div>
        )}
      </div>
    </nav>
  )
}
