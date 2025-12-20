import { Github, MessageSquare, Star, Twitter } from 'lucide-react'

const communityLinks = [
  {
    icon: Github,
    title: 'GitHub',
    description: 'Star the repo, report issues, and contribute code.',
    href: 'https://github.com/ZingerLittleBee/EXLO',
    cta: '[ VIEW REPO ]'
  },
  {
    icon: MessageSquare,
    title: 'Discussions',
    description: 'Join our community server for discussions and support.',
    href: 'https://github.com/ZingerLittleBee/EXLO/discussions',
    cta: '[ JOIN DISCUSSIONS ]'
  },
  {
    icon: Twitter,
    title: 'Twitter',
    description: 'Follow for updates, tips, and announcements.',
    href: 'https://twitter.com/zinger_bee',
    cta: '[ FOLLOW US ]'
  }
]

export function CommunitySection() {
  return (
    <section className="border-border border-t px-4 py-20 sm:px-6 lg:px-8">
      <div className="mx-auto max-w-6xl">
        {/* Section header */}
        <div className="mb-16 text-center">
          <div className="mb-6 inline-block border border-border px-4 py-2">
            <span className="text-muted-foreground text-xs uppercase tracking-widest">$ whois --community</span>
          </div>
          <h2 className="terminal-glow font-bold text-2xl uppercase tracking-tight sm:text-3xl">Join the Community</h2>
          <p className="mx-auto mt-4 max-w-xl text-muted-foreground text-sm">
            Connect with developers who are building and using EXLO around the world.
          </p>
          <div className="mx-auto mt-6 h-px w-32 bg-border" />
        </div>

        {/* Community links */}
        <div className="grid grid-cols-1 gap-6 md:grid-cols-3">
          {communityLinks.map((link, index) => (
            <a
              className="group block border border-border p-6 transition-colors hover:border-primary"
              href={link.href}
              key={index}
              rel="noopener noreferrer"
              target="_blank"
            >
              <div className="mb-4 flex items-center gap-3">
                <link.icon className="h-6 w-6 text-primary" strokeWidth={2} />
                <h3 className="font-bold uppercase tracking-wider">{link.title}</h3>
              </div>
              <p className="mb-6 text-muted-foreground text-sm">{link.description}</p>
              <span className="group-hover:terminal-glow text-primary text-xs transition-all">{link.cta}</span>
            </a>
          ))}
        </div>

        {/* Star CTA */}
        <div className="mt-16 border border-border p-8 text-center">
          <div className="mb-4 flex items-center justify-center gap-2">
            <Star className="h-6 w-6 fill-secondary text-secondary" />
            {/* <span className="terminal-glow font-bold text-2xl">1,234+</span> */}
            <span className="text-muted-foreground text-sm uppercase">GitHub Stars</span>
          </div>
          <p className="mb-6 text-muted-foreground text-sm">Show your support by starring the repository on GitHub</p>
          <a
            className="inline-block bg-primary px-6 py-3 font-bold text-primary-foreground text-sm uppercase tracking-wider transition-colors hover:bg-primary/90"
            href="https://github.com/ZingerLittleBee/EXLO"
            rel="noopener noreferrer"
            target="_blank"
          >
            [ ★ STAR ON GITHUB ]
          </a>
        </div>
      </div>
    </section>
  )
}
