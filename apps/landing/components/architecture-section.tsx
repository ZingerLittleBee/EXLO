import { ArrowRight, Server, Terminal, Zap } from 'lucide-react'

const steps = [
  {
    icon: Terminal,
    step: '01',
    title: 'Terminal',
    description: 'User runs ssh -R from any machine with OpenSSH installed.'
  },
  {
    icon: Server,
    step: '02',
    title: 'Gatekeeper',
    description: 'Node.js web server initiates device flow authentication.'
  },
  {
    icon: Zap,
    step: '03',
    title: 'Tunnel',
    description: 'Rust server establishes high-performance stream connection.'
  }
]

export function ArchitectureSection() {
  return (
    <section className="border-border border-t bg-background/50 px-4 py-20 sm:px-6 lg:px-8">
      <div className="mx-auto max-w-6xl">
        {/* Section header */}
        <div className="mb-16 text-center">
          <div className="mb-6 inline-block border border-border px-4 py-2">
            <span className="text-muted-foreground text-xs uppercase tracking-widest">$ ./explain --architecture</span>
          </div>
          <h2 className="terminal-glow font-bold text-2xl uppercase tracking-tight sm:text-3xl">How It Works</h2>
          <div className="mx-auto mt-6 h-px w-32 bg-border" />
        </div>

        {/* Architecture diagram */}
        <div className="relative grid grid-cols-1 gap-8 md:grid-cols-3">
          {steps.map((step, index) => (
            <div className="relative" key={index}>
              {/* Connection arrow (desktop only) */}
              {index < steps.length - 1 && (
                <div className="absolute top-1/2 -right-4 z-10 hidden -translate-y-1/2 transform md:block">
                  <ArrowRight className="terminal-glow h-8 w-8 text-primary" />
                </div>
              )}

              <div className="h-full border border-border p-6">
                {/* Step number */}
                <div className="mb-4 flex items-center justify-between">
                  <span className="text-muted-foreground text-xs">STEP {step.step}</span>
                  <step.icon className="h-6 w-6 text-primary" strokeWidth={2} />
                </div>

                {/* Title */}
                <h3 className="terminal-glow mb-3 font-bold text-lg uppercase tracking-wider">{step.title}</h3>

                {/* Separator */}
                <div className="mb-3 text-muted-foreground text-xs">================</div>

                {/* Description */}
                <p className="text-muted-foreground text-sm leading-relaxed">{step.description}</p>
              </div>
            </div>
          ))}
        </div>

        {/* ASCII art diagram */}
        <div className="mt-12 overflow-x-auto border border-border p-6">
          <pre className="whitespace-pre text-center text-muted-foreground text-xs sm:text-sm">
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
                 https://demo.youdomain
`}
          </pre>
        </div>
      </div>
    </section>
  )
}
