export function Footer() {
  const currentYear = new Date().getFullYear()

  return (
    <footer className="border-t border-border py-12 px-4 sm:px-6 lg:px-8">
      <div className="max-w-6xl mx-auto">
        <div className="text-center mb-8">
          <pre className="text-xs text-muted-foreground inline-block text-left">
            {`
  __             _                
 / _|_      ____| |  _ __ ___ 
| |_\\ \\ /\\ / / _\` | | '__/ __|
|  _|\\ V  V / (_| |_| |  \\__ \\
|_|   \\_/\\_/ \\__,_(_)_|  |___/
                              
`}
          </pre>
        </div>

        {/* Footer links - Updated all URLs from open-tunnl to fwd-rs */}
        <div className="grid grid-cols-2 md:grid-cols-4 gap-8 mb-12">
          <div>
            <h4 className="text-xs font-bold uppercase tracking-wider mb-4">Project</h4>
            <ul className="space-y-2 text-xs text-muted-foreground">
              <li>
                <a href="#features" className="hover:text-foreground transition-colors">
                  {">"} Features
                </a>
              </li>
              <li>
                <a href="#docs" className="hover:text-foreground transition-colors">
                  {">"} Documentation
                </a>
              </li>
              <li>
                <a
                  href="https://github.com/fwd-rs/fwd.rs/releases"
                  target="_blank"
                  rel="noopener noreferrer"
                  className="hover:text-foreground transition-colors"
                >
                  {">"} Releases
                </a>
              </li>
            </ul>
          </div>
          <div>
            <h4 className="text-xs font-bold uppercase tracking-wider mb-4">Resources</h4>
            <ul className="space-y-2 text-xs text-muted-foreground">
              <li>
                <a
                  href="https://github.com/fwd-rs/fwd.rs/wiki"
                  target="_blank"
                  rel="noopener noreferrer"
                  className="hover:text-foreground transition-colors"
                >
                  {">"} Wiki
                </a>
              </li>
              <li>
                <a
                  href="https://github.com/fwd-rs/fwd.rs/blob/main/CHANGELOG.md"
                  target="_blank"
                  rel="noopener noreferrer"
                  className="hover:text-foreground transition-colors"
                >
                  {">"} Changelog
                </a>
              </li>
              <li>
                <a
                  href="https://github.com/fwd-rs/fwd.rs/issues"
                  target="_blank"
                  rel="noopener noreferrer"
                  className="hover:text-foreground transition-colors"
                >
                  {">"} Issues
                </a>
              </li>
            </ul>
          </div>
          <div>
            <h4 className="text-xs font-bold uppercase tracking-wider mb-4">Community</h4>
            <ul className="space-y-2 text-xs text-muted-foreground">
              <li>
                <a
                  href="https://github.com/fwd-rs/fwd.rs"
                  target="_blank"
                  rel="noopener noreferrer"
                  className="hover:text-foreground transition-colors"
                >
                  {">"} GitHub
                </a>
              </li>
              <li>
                <a
                  href="https://discord.gg/fwd-rs"
                  target="_blank"
                  rel="noopener noreferrer"
                  className="hover:text-foreground transition-colors"
                >
                  {">"} Discord
                </a>
              </li>
              <li>
                <a
                  href="https://twitter.com/fwd_rs"
                  target="_blank"
                  rel="noopener noreferrer"
                  className="hover:text-foreground transition-colors"
                >
                  {">"} Twitter
                </a>
              </li>
            </ul>
          </div>
          <div>
            <h4 className="text-xs font-bold uppercase tracking-wider mb-4">Legal</h4>
            <ul className="space-y-2 text-xs text-muted-foreground">
              <li>
                <a
                  href="https://github.com/fwd-rs/fwd.rs/blob/main/LICENSE"
                  target="_blank"
                  rel="noopener noreferrer"
                  className="hover:text-foreground transition-colors"
                >
                  {">"} MIT License
                </a>
              </li>
              <li>
                <a
                  href="https://github.com/fwd-rs/fwd.rs/blob/main/CODE_OF_CONDUCT.md"
                  target="_blank"
                  rel="noopener noreferrer"
                  className="hover:text-foreground transition-colors"
                >
                  {">"} Code of Conduct
                </a>
              </li>
              <li>
                <a
                  href="https://github.com/fwd-rs/fwd.rs/security"
                  target="_blank"
                  rel="noopener noreferrer"
                  className="hover:text-foreground transition-colors"
                >
                  {">"} Security
                </a>
              </li>
            </ul>
          </div>
        </div>

        {/* Bottom bar - Updated copyright from Open Tunnl to fwd.rs */}
        <div className="border-t border-border pt-8 flex flex-col sm:flex-row items-center justify-between gap-4">
          <div className="flex items-center gap-2 text-xs text-muted-foreground">
            <span className="w-2 h-2 bg-primary animate-pulse" />
            <span>fwd.rs © {currentYear}</span>
            <span>|</span>
            <span>Released under MIT License</span>
          </div>
          <div className="text-xs text-muted-foreground">
            Made with <span className="text-destructive">{"<3"}</span> by the open-source community
          </div>
        </div>

        {/* Terminal prompt - Updated prompt from open-tunnl to fwd.rs */}
        <div className="mt-8 text-center">
          <code className="text-xs text-muted-foreground">
            user@fwd.rs:~$ exit <span className="animate-blink">█</span>
          </code>
        </div>
      </div>
    </footer>
  )
}
