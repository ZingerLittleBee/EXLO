# Contributing to EXLO

Thank you for your interest in this project! We welcome contributions of all kinds.

## Development Environment Setup

### Prerequisites

- [Bun](https://bun.sh/) v1.3.4 or higher
- [Node.js](https://nodejs.org/) v18+
- [Rust](https://www.rust-lang.org/) (for Data Plane development)
- [Docker](https://www.docker.com/) (optional, for running PostgreSQL)

### Install Dependencies

```bash
bun install
```

### Start Development Server

```bash
# Start all applications
bun dev

# Or start specific applications
bun dev:web      # Web application
bun dev:docs     # Documentation site
bun dev:landing  # Landing page
```

### Database Operations

```bash
bun db:start     # Start database container
bun db:push      # Push schema changes
bun db:studio    # Open Drizzle Studio
bun db:stop      # Stop database container
```

## Code Standards

This project uses [Ultracite](https://github.com/haydenbleasel/ultracite) (based on Biome) for code formatting and linting.

### Check Code

```bash
bun check
```

### Auto Fix

```bash
bun fix
```

> [!IMPORTANT]
> Please run `bun fix` before submitting a PR to ensure your code meets the standards.

## Project Structure

```
.
├── apps/
│   ├── web/        # TanStack Start Web application
│   ├── docs/       # Documentation site
│   ├── landing/    # Landing page
│   └── tunnl/      # Rust SSH tunnel service
├── packages/       # Shared packages
│   └── db/         # Database schema and configuration
└── ...
```

## Commit Guidelines

We recommend following the [Conventional Commits](https://www.conventionalcommits.org/) specification:

```
<type>(<scope>): <description>

[optional body]
```

### Common Types

- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation update
- `style`: Code formatting adjustments
- `refactor`: Code refactoring
- `test`: Test-related changes
- `chore`: Build/tooling changes

### Examples

```
feat(web): add tunnel status dashboard
fix(tunnl): resolve connection timeout issue
docs: update README with new setup instructions
```

## Pull Request Process

1. Fork this repository
2. Create a feature branch: `git checkout -b feat/amazing-feature`
3. Commit your changes: `git commit -m 'feat: add amazing feature'`
4. Push the branch: `git push origin feat/amazing-feature`
5. Submit a Pull Request

## Development Guide

### Web Application (TanStack Start)

- Framework: TanStack Start (SSR)
- Authentication: Better Auth
- ORM: Drizzle

### Rust Service

- SSH Service: `russh`
- HTTP Proxy: `hyper`
- Management API: `axum`

## Issue Reporting

If you find a bug or have a feature suggestion, please submit it via [Issues](https://github.com/ZingerLittleBee/EXLO/issues).

---

Thank you again for your contribution! 🎉
