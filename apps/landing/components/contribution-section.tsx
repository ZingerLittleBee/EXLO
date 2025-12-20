import { GitBranch, Bug, BookOpen, Users } from "lucide-react"

const contributionWays = [
  {
    icon: Bug,
    title: "Report Issues",
    description: "Found a bug? Open an issue on GitHub with steps to reproduce.",
    command: "gh issue create",
  },
  {
    icon: GitBranch,
    title: "Submit PRs",
    description: "Fork the repo, make changes, and submit a pull request.",
    command: "git checkout -b feature/awesome",
  },
  {
    icon: BookOpen,
    title: "Improve Docs",
    description: "Help us improve documentation and tutorials.",
    command: "vim docs/README.md",
  },
  {
    icon: Users,
    title: "Join Discord",
    description: "Connect with other contributors and maintainers.",
    command: "discord.gg/fwd-rs",
  },
]

export function ContributionSection() {
  return (
    <section className="py-20 px-4 sm:px-6 lg:px-8 border-t border-border bg-background/50">
      <div className="max-w-6xl mx-auto">
        {/* Section header */}
        <div className="text-center mb-16">
          <div className="inline-block border border-border px-4 py-2 mb-6">
            <span className="text-xs text-muted-foreground uppercase tracking-widest">
              $ git commit -m &quot;Add awesome feature&quot;
            </span>
          </div>
          <h2 className="text-2xl sm:text-3xl font-bold uppercase tracking-tight terminal-glow">
            Contribute to fwd.rs
          </h2>
          <p className="text-muted-foreground text-sm mt-4 max-w-xl mx-auto">
            Open source thrives on community contributions. Here&apos;s how you can help make fwd.rs better.
          </p>
          <div className="w-32 h-px bg-border mx-auto mt-6" />
        </div>

        {/* Contribution ways */}
        <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
          {contributionWays.map((way, index) => (
            <div key={index} className="border border-border p-6 hover:border-primary transition-colors group">
              <div className="flex items-start gap-4">
                <div className="p-3 border border-border group-hover:border-primary transition-colors">
                  <way.icon className="w-6 h-6 text-primary" strokeWidth={2} />
                </div>
                <div className="flex-1">
                  <h3 className="font-bold uppercase tracking-wider text-sm mb-2">{way.title}</h3>
                  <p className="text-sm text-muted-foreground mb-4">{way.description}</p>
                  <code className="text-xs text-secondary">$ {way.command}</code>
                </div>
              </div>
            </div>
          ))}
        </div>

        {/* Contribution guidelines */}
        <div className="mt-12 border border-border p-6">
          <div className="flex items-center gap-2 mb-4">
            <span className="text-primary">{">"}</span>
            <span className="text-sm font-bold uppercase tracking-wider">CONTRIBUTING.md</span>
          </div>
          <div className="text-xs text-muted-foreground space-y-2">
            <p>1. Fork the repository and create your branch from `main`.</p>
            <p>2. If you&apos;ve added code that should be tested, add tests.</p>
            <p>3. Ensure the test suite passes with `cargo test`.</p>
            <p>4. Make sure your code follows the existing style (`cargo fmt`).</p>
            <p>5. Issue that pull request!</p>
          </div>
          <div className="mt-6 pt-4 border-t border-border">
            <a
              href="https://github.com/fwd-rs/fwd.rs/blob/main/CONTRIBUTING.md"
              target="_blank"
              rel="noopener noreferrer"
              className="inline-block px-4 py-2 border border-primary text-primary hover:bg-primary hover:text-primary-foreground transition-colors text-xs uppercase tracking-wider"
            >
              [ Read Full Guidelines ]
            </a>
          </div>
        </div>
      </div>
    </section>
  )
}
