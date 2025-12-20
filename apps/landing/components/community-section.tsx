import { Github, MessageSquare, Twitter, Star } from "lucide-react"

const communityLinks = [
  {
    icon: Github,
    title: "GitHub",
    description: "Star the repo, report issues, and contribute code.",
    href: "https://github.com/fwd-rs/fwd.rs",
    cta: "[ VIEW REPO ]",
  },
  {
    icon: MessageSquare,
    title: "Discord",
    description: "Join our community server for discussions and support.",
    href: "https://discord.gg/fwd-rs",
    cta: "[ JOIN SERVER ]",
  },
  {
    icon: Twitter,
    title: "Twitter",
    description: "Follow for updates, tips, and announcements.",
    href: "https://twitter.com/fwd_rs",
    cta: "[ FOLLOW US ]",
  },
]

export function CommunitySection() {
  return (
    <section className="py-20 px-4 sm:px-6 lg:px-8 border-t border-border">
      <div className="max-w-6xl mx-auto">
        {/* Section header */}
        <div className="text-center mb-16">
          <div className="inline-block border border-border px-4 py-2 mb-6">
            <span className="text-xs text-muted-foreground uppercase tracking-widest">$ whois --community</span>
          </div>
          <h2 className="text-2xl sm:text-3xl font-bold uppercase tracking-tight terminal-glow">Join the Community</h2>
          <p className="text-muted-foreground text-sm mt-4 max-w-xl mx-auto">
            Connect with developers who are building and using fwd.rs around the world.
          </p>
          <div className="w-32 h-px bg-border mx-auto mt-6" />
        </div>

        {/* Community links */}
        <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
          {communityLinks.map((link, index) => (
            <a
              key={index}
              href={link.href}
              target="_blank"
              rel="noopener noreferrer"
              className="border border-border p-6 hover:border-primary transition-colors group block"
            >
              <div className="flex items-center gap-3 mb-4">
                <link.icon className="w-6 h-6 text-primary" strokeWidth={2} />
                <h3 className="font-bold uppercase tracking-wider">{link.title}</h3>
              </div>
              <p className="text-sm text-muted-foreground mb-6">{link.description}</p>
              <span className="text-xs text-primary group-hover:terminal-glow transition-all">{link.cta}</span>
            </a>
          ))}
        </div>

        {/* Star CTA */}
        <div className="mt-16 text-center border border-border p-8">
          <div className="flex items-center justify-center gap-2 mb-4">
            <Star className="w-6 h-6 text-secondary fill-secondary" />
            <span className="text-2xl font-bold terminal-glow">1,234+</span>
            <span className="text-muted-foreground uppercase text-sm">GitHub Stars</span>
          </div>
          <p className="text-muted-foreground text-sm mb-6">Show your support by starring the repository on GitHub</p>
          <a
            href="https://github.com/fwd-rs/fwd.rs"
            target="_blank"
            rel="noopener noreferrer"
            className="inline-block px-6 py-3 bg-primary text-primary-foreground font-bold uppercase tracking-wider hover:bg-primary/90 transition-colors text-sm"
          >
            [ ★ STAR ON GITHUB ]
          </a>
        </div>
      </div>
    </section>
  )
}
