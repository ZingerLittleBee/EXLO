import { Analytics } from '@vercel/analytics/next'
import type { Metadata } from 'next'
import { JetBrains_Mono } from 'next/font/google'
import type React from 'react'
import './globals.css'

const jetbrainsMono = JetBrains_Mono({
  subsets: ['latin'],
  variable: '--font-mono'
})

export const metadata: Metadata = {
  title: 'fwd.rs - Self-Hosted SSH Reverse Tunnel',
  description:
    'The self-hosted, clientless alternative to ngrok. Expose localhost to the internet securely with full control over your data.',
  keywords: ['ssh', 'tunnel', 'reverse proxy', 'ngrok alternative', 'self-hosted', 'open source']
}

export default function RootLayout({
  children
}: Readonly<{
  children: React.ReactNode
}>) {
  return (
    <html className="dark" lang="en">
      <link href="/favicon.ico" rel="icon" />
      <body className={`${jetbrainsMono.className} antialiased`}>
        {children}
        <Analytics />
      </body>
    </html>
  )
}
