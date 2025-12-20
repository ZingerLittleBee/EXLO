import { Terminal, Shield, Zap, Eye, Server, Lock } from "lucide-react"

const features = [
  {
    icon: Terminal,
    title: "Zero Config",
    description: "Works with standard OpenSSH installed on every machine. No client installation required.",
    status: "[OK]",
  },
  {
    icon: Shield,
    title: "Device Flow Auth",
    description: "Secure, browser-based authentication for every session. OAuth 2.0 device flow.",
    status: "[OK]",
  },
  {
    icon: Zap,
    title: "Rust Core",
    description: "Built on russh and tokio for high-performance async concurrency and minimal overhead.",
    status: "[OK]",
  },
  {
    icon: Eye,
    title: "God Mode",
    description: "Admins can monitor active connections and terminate suspicious tunnels instantly.",
    status: "[OK]",
  },
  {
    icon: Server,
    title: "Virtual Ports",
    description: "Smart port handling - bind to port 80, forward to any local port seamlessly.",
    status: "[OK]",
  },
  {
    icon: Lock,
    title: "Private & Secure",
    description: "Data never leaves your infrastructure. TLS termination and end-to-end encryption included.",
    status: "[OK]",
  },
]

export function FeaturesSection() {
  return (
    <section id="features" className="py-20 px-4 sm:px-6 lg:px-8 border-t border-border">
      <div className="max-w-6xl mx-auto">
        {/* Section header */}
        <div className="text-center mb-16">
          <div className="inline-block border border-border px-4 py-2 mb-6">
            <span className="text-xs text-muted-foreground uppercase tracking-widest">$ cat features.txt</span>
          </div>
          <h2 className="text-2xl sm:text-3xl font-bold uppercase tracking-tight terminal-glow">Key Features</h2>
          <div className="w-32 h-px bg-border mx-auto mt-6" />
        </div>

        {/* Features grid */}
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
          {features.map((feature, index) => (
            <div key={index} className="border border-border p-6 hover:border-primary transition-colors group">
              {/* Feature header */}
              <div className="flex items-center justify-between mb-4">
                <div className="flex items-center gap-3">
                  <feature.icon className="w-5 h-5 text-primary" strokeWidth={2} />
                  <h3 className="font-bold uppercase tracking-wider text-sm">{feature.title}</h3>
                </div>
                <span className="text-xs text-primary terminal-glow">{feature.status}</span>
              </div>

              {/* Separator */}
              <div className="text-muted-foreground text-xs mb-4">--------------------------------</div>

              {/* Description */}
              <p className="text-sm text-muted-foreground leading-relaxed">{feature.description}</p>
            </div>
          ))}
        </div>
      </div>
    </section>
  )
}
