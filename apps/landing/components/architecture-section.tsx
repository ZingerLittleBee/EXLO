import { Terminal, Server, Zap, ArrowRight } from "lucide-react"

const steps = [
  {
    icon: Terminal,
    step: "01",
    title: "Terminal",
    description: "User runs ssh -R from any machine with OpenSSH installed.",
  },
  {
    icon: Server,
    step: "02",
    title: "Gatekeeper",
    description: "Node.js web server initiates device flow authentication.",
  },
  {
    icon: Zap,
    step: "03",
    title: "Tunnel",
    description: "Rust server establishes high-performance stream connection.",
  },
]

export function ArchitectureSection() {
  return (
    <section className="py-20 px-4 sm:px-6 lg:px-8 border-t border-border bg-background/50">
      <div className="max-w-6xl mx-auto">
        {/* Section header */}
        <div className="text-center mb-16">
          <div className="inline-block border border-border px-4 py-2 mb-6">
            <span className="text-xs text-muted-foreground uppercase tracking-widest">$ ./explain --architecture</span>
          </div>
          <h2 className="text-2xl sm:text-3xl font-bold uppercase tracking-tight terminal-glow">How It Works</h2>
          <div className="w-32 h-px bg-border mx-auto mt-6" />
        </div>

        {/* Architecture diagram */}
        <div className="grid grid-cols-1 md:grid-cols-3 gap-8 relative">
          {steps.map((step, index) => (
            <div key={index} className="relative">
              {/* Connection arrow (desktop only) */}
              {index < steps.length - 1 && (
                <div className="hidden md:block absolute top-1/2 -right-4 transform -translate-y-1/2 z-10">
                  <ArrowRight className="w-8 h-8 text-primary terminal-glow" />
                </div>
              )}

              <div className="border border-border p-6 h-full">
                {/* Step number */}
                <div className="flex items-center justify-between mb-4">
                  <span className="text-xs text-muted-foreground">STEP {step.step}</span>
                  <step.icon className="w-6 h-6 text-primary" strokeWidth={2} />
                </div>

                {/* Title */}
                <h3 className="text-lg font-bold uppercase tracking-wider mb-3 terminal-glow">{step.title}</h3>

                {/* Separator */}
                <div className="text-muted-foreground text-xs mb-3">================</div>

                {/* Description */}
                <p className="text-sm text-muted-foreground leading-relaxed">{step.description}</p>
              </div>
            </div>
          ))}
        </div>

        {/* ASCII art diagram */}
        <div className="mt-12 border border-border p-6 overflow-x-auto">
          <pre className="text-xs sm:text-sm text-muted-foreground whitespace-pre text-center">
            {`
┌─────────────┐      ┌─────────────┐      ┌─────────────┐
│  YOUR DEV   │      │  GATEKEEPER │      │   TUNNEL    │
│   MACHINE   │─────▶│   NODE.JS   │─────▶│    RUST     │
│  (ssh -R)   │      │  (auth)     │      │  (stream)   │
└─────────────┘      └─────────────┘      └─────────────┘
       │                                         │
       │                                         │
       └─────────── localhost:3000 ◀─────────────┘
                    EXPOSED AS
                 https://demo.fwd.rs
`}
          </pre>
        </div>
      </div>
    </section>
  )
}
