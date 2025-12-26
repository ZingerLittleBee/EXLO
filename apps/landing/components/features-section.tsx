import { Eye, Lock, Server, Shield, Terminal, Zap } from 'lucide-react'

const features = [
  {
    icon: Terminal,
    title: 'No Client Required',
    description: 'Works with the OpenSSH already on your machine. No downloads, no installations, no agents.',
    status: '[OK]'
  },
  {
    icon: Shield,
    title: 'Device Flow Auth',
    description: 'Secure, browser-based authentication for every session. OAuth 2.0 device flow.',
    status: '[OK]'
  },
  {
    icon: Zap,
    title: 'Rust Core',
    description: 'Built on russh and tokio for high-performance async concurrency and minimal overhead.',
    status: '[OK]'
  },
  {
    icon: Eye,
    title: 'God Mode',
    description: 'Admins can monitor active connections and terminate suspicious tunnels instantly.',
    status: '[OK]'
  },
  {
    icon: Server,
    title: 'Virtual Ports',
    description: 'Smart port handling - bind to port 80, forward to any local port seamlessly.',
    status: '[OK]'
  },
  {
    icon: Lock,
    title: 'Private & Secure',
    description: 'Data never leaves your infrastructure. TLS termination and end-to-end encryption included.',
    status: '[OK]'
  }
]

export function FeaturesSection() {
  return (
    <section className="border-border border-t px-4 py-20 sm:px-6 lg:px-8" id="features">
      <div className="mx-auto max-w-6xl">
        {/* Section header */}
        <div className="mb-16 text-center">
          <div className="mb-6 inline-block border border-border px-4 py-2">
            <span className="text-muted-foreground text-xs uppercase tracking-widest">$ cat features.txt</span>
          </div>
          <h2 className="terminal-glow font-bold text-2xl uppercase tracking-tight sm:text-3xl">Key Features</h2>
          <div className="mx-auto mt-6 h-px w-32 bg-border" />
        </div>

        {/* Features grid */}
        <div className="grid grid-cols-1 gap-6 md:grid-cols-2 lg:grid-cols-3">
          {features.map((feature, index) => (
            <div className="group border border-border p-6 transition-colors hover:border-primary" key={index}>
              {/* Feature header */}
              <div className="mb-4 flex items-center justify-between">
                <div className="flex items-center gap-3">
                  <feature.icon className="h-5 w-5 text-primary" strokeWidth={2} />
                  <h3 className="font-bold text-sm uppercase tracking-wider">{feature.title}</h3>
                </div>
                <span className="terminal-glow text-primary text-xs">{feature.status}</span>
              </div>

              {/* Separator */}
              <div className="mb-4 text-muted-foreground text-xs">--------------------------------</div>

              {/* Description */}
              <p className="text-muted-foreground text-sm leading-relaxed">{feature.description}</p>
            </div>
          ))}
        </div>
      </div>
    </section>
  )
}
