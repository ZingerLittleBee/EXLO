import { BookOpen, Bug, GitBranch, Users } from 'lucide-react'

const contributionWays = [
  {
    icon: Bug,
    title: 'Report Issues',
    description: 'Found a bug? Open an issue on GitHub with steps to reproduce.',
    command: 'gh issue create'
  },
  {
    icon: GitBranch,
    title: 'Submit PRs',
    description: 'Fork the repo, make changes, and submit a pull request.',
    command: 'git checkout -b feature/awesome'
  },
  {
    icon: BookOpen,
    title: 'Improve Docs',
    description: 'Help us improve documentation and tutorials.',
    command: 'vim docs/README.md'
  },
  {
    icon: Users,
    title: 'Join Discord',
    description: 'Connect with other contributors and maintainers.',
    command: 'discord.gg/fwd-rs'
  }
]

export function ContributionSection() {
  return (
    <section className="border-border border-t bg-background/50 px-4 py-20 sm:px-6 lg:px-8">
      <div className="mx-auto max-w-6xl">
        {/* Section header */}
        <div className="mb-16 text-center">
          <div className="mb-6 inline-block border border-border px-4 py-2">
            <span className="text-muted-foreground text-xs uppercase tracking-widest">
              $ git commit -m &quot;Add awesome feature&quot;
            </span>
          </div>
          <h2 className="terminal-glow font-bold text-2xl uppercase tracking-tight sm:text-3xl">Contribute to EXLO</h2>
          <p className="mx-auto mt-4 max-w-xl text-muted-foreground text-sm">
            Open source thrives on community contributions. Here&apos;s how you can help make EXLO better.
          </p>
          <div className="mx-auto mt-6 h-px w-32 bg-border" />
        </div>

        {/* Contribution ways */}
        <div className="grid grid-cols-1 gap-6 md:grid-cols-2">
          {contributionWays.map((way, index) => (
            <div className="group border border-border p-6 transition-colors hover:border-primary" key={index}>
              <div className="flex items-start gap-4">
                <div className="border border-border p-3 transition-colors group-hover:border-primary">
                  <way.icon className="h-6 w-6 text-primary" strokeWidth={2} />
                </div>
                <div className="flex-1">
                  <h3 className="mb-2 font-bold text-sm uppercase tracking-wider">{way.title}</h3>
                  <p className="mb-4 text-muted-foreground text-sm">{way.description}</p>
                  <code className="text-secondary text-xs">$ {way.command}</code>
                </div>
              </div>
            </div>
          ))}
        </div>

        {/* Contribution guidelines */}
        <div className="mt-12 border border-border p-6">
          <div className="mb-4 flex items-center gap-2">
            <span className="text-primary">{'>'}</span>
            <span className="font-bold text-sm uppercase tracking-wider">CONTRIBUTING.md</span>
          </div>
          <div className="space-y-2 text-muted-foreground text-xs">
            <p>1. Fork the repository and create your branch from `main`.</p>
            <p>2. If you&apos;ve added code that should be tested, add tests.</p>
            <p>3. Ensure the test suite passes with `cargo test`.</p>
            <p>4. Make sure your code follows the existing style (`cargo fmt`).</p>
            <p>5. Issue that pull request!</p>
          </div>
          <div className="mt-6 border-border border-t pt-4">
            <a
              className="inline-block border border-primary px-4 py-2 text-primary text-xs uppercase tracking-wider transition-colors hover:bg-primary hover:text-primary-foreground"
              href="https://github.com/ZingerLittleBee/EXLO/blob/main/CONTRIBUTING.md"
              rel="noopener noreferrer"
              target="_blank"
            >
              [ Read Full Guidelines ]
            </a>
          </div>
        </div>
      </div>
    </section>
  )
}
