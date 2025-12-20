export function Footer() {
  const currentYear = new Date().getFullYear()

  return (
    <footer className="border-border border-t px-4 py-12 sm:px-6 lg:px-8">
      <div className="mx-auto max-w-6xl">
        <div className="mb-12 text-center">
          <pre className="inline-block text-left text-muted-foreground text-xs">
            {/* https://patorjk.com/software/taag/#p=display&f=Big+Money-nw&t=FWD.RS&x=none&v=4&h=4&w=80&we=false */}
            {`$$$$$$$$\\ $$\\      $$\\ $$$$$$$\\      $$$$$$$\\   $$$$$$\\  
$$  _____|$$ | $\\  $$ |$$  __$$\\     $$  __$$\\ $$  __$$\\ 
$$ |      $$ |$$$\\ $$ |$$ |  $$ |    $$ |  $$ |$$ /  \\__|
$$$$$\\    $$ $$ $$\\$$ |$$ |  $$ |    $$$$$$$  |\\$$$$$$\\  
$$  __|   $$$$  _$$$$ |$$ |  $$ |    $$  __$$<  \\____$$\\ 
$$ |      $$$  / \\$$$ |$$ |  $$ |    $$ |  $$ |$$\\   $$ |
$$ |      $$  /   \\$$ |$$$$$$$  |$$\\ $$ |  $$ |\\$$$$$$  |
\\__|      \\__/     \\__|\\_______/ \\__|\\__|  \\__| \\______/ 
`}
          </pre>
        </div>

        {/* Footer links - Updated all URLs from open-tunnl to fwd-rs */}
        <div className="mb-12 grid grid-cols-2 gap-8 md:grid-cols-4">
          <div>
            <h4 className="mb-4 font-bold text-xs uppercase tracking-wider">Project</h4>
            <ul className="space-y-2 text-muted-foreground text-xs">
              <li>
                <a className="transition-colors hover:text-foreground" href="#features">
                  {'>'} Features
                </a>
              </li>
              <li>
                <a className="transition-colors hover:text-foreground" href="#docs">
                  {'>'} Documentation
                </a>
              </li>
              <li>
                <a
                  className="transition-colors hover:text-foreground"
                  href="https://github.com/fwd-rs/fwd.rs/releases"
                  rel="noopener noreferrer"
                  target="_blank"
                >
                  {'>'} Releases
                </a>
              </li>
            </ul>
          </div>
          <div>
            <h4 className="mb-4 font-bold text-xs uppercase tracking-wider">Resources</h4>
            <ul className="space-y-2 text-muted-foreground text-xs">
              <li>
                <a
                  className="transition-colors hover:text-foreground"
                  href="https://github.com/fwd-rs/fwd.rs/wiki"
                  rel="noopener noreferrer"
                  target="_blank"
                >
                  {'>'} Wiki
                </a>
              </li>
              <li>
                <a
                  className="transition-colors hover:text-foreground"
                  href="https://github.com/fwd-rs/fwd.rs/blob/main/CHANGELOG.md"
                  rel="noopener noreferrer"
                  target="_blank"
                >
                  {'>'} Changelog
                </a>
              </li>
              <li>
                <a
                  className="transition-colors hover:text-foreground"
                  href="https://github.com/fwd-rs/fwd.rs/issues"
                  rel="noopener noreferrer"
                  target="_blank"
                >
                  {'>'} Issues
                </a>
              </li>
            </ul>
          </div>
          <div>
            <h4 className="mb-4 font-bold text-xs uppercase tracking-wider">Community</h4>
            <ul className="space-y-2 text-muted-foreground text-xs">
              <li>
                <a
                  className="transition-colors hover:text-foreground"
                  href="https://github.com/fwd-rs/fwd.rs"
                  rel="noopener noreferrer"
                  target="_blank"
                >
                  {'>'} GitHub
                </a>
              </li>
              <li>
                <a
                  className="transition-colors hover:text-foreground"
                  href="https://discord.gg/fwd-rs"
                  rel="noopener noreferrer"
                  target="_blank"
                >
                  {'>'} Discord
                </a>
              </li>
              <li>
                <a
                  className="transition-colors hover:text-foreground"
                  href="https://twitter.com/fwd_rs"
                  rel="noopener noreferrer"
                  target="_blank"
                >
                  {'>'} Twitter
                </a>
              </li>
            </ul>
          </div>
          <div>
            <h4 className="mb-4 font-bold text-xs uppercase tracking-wider">Legal</h4>
            <ul className="space-y-2 text-muted-foreground text-xs">
              <li>
                <a
                  className="transition-colors hover:text-foreground"
                  href="https://github.com/fwd-rs/fwd.rs/blob/main/LICENSE"
                  rel="noopener noreferrer"
                  target="_blank"
                >
                  {'>'} MIT License
                </a>
              </li>
              <li>
                <a
                  className="transition-colors hover:text-foreground"
                  href="https://github.com/fwd-rs/fwd.rs/blob/main/CODE_OF_CONDUCT.md"
                  rel="noopener noreferrer"
                  target="_blank"
                >
                  {'>'} Code of Conduct
                </a>
              </li>
              <li>
                <a
                  className="transition-colors hover:text-foreground"
                  href="https://github.com/fwd-rs/fwd.rs/security"
                  rel="noopener noreferrer"
                  target="_blank"
                >
                  {'>'} Security
                </a>
              </li>
            </ul>
          </div>
        </div>

        {/* Bottom bar - Updated copyright from Open Tunnl to fwd.rs */}
        <div className="flex flex-col items-center justify-between gap-4 border-border border-t pt-8 sm:flex-row">
          <div className="flex items-center gap-2 text-muted-foreground text-xs">
            <span className="h-2 w-2 animate-pulse bg-primary" />
            <span>fwd.rs © {currentYear}</span>
            <span>|</span>
            <span>Released under MIT License</span>
          </div>
          <div className="text-muted-foreground text-xs">
            Made with{' '}
            <a className="text-destructive" href="https://x.com/zinger_bee" rel="noopener noreferrer" target="_blank">
              {'ZingerBee'}
            </a>{' '}
            by the open-source community
          </div>
        </div>

        {/* Terminal prompt - Updated prompt from open-tunnl to fwd.rs */}
        <div className="mt-8 text-center">
          <code className="text-muted-foreground text-xs">
            user@fwd.rs:~$ exit <span className="animate-blink">█</span>
          </code>
        </div>
      </div>
    </footer>
  )
}
