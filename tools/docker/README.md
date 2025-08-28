# Docker Development Environment

Zero‑config Rust development without a local Rust install.

## Quick Start

```bash
cd tools/docker

# Run all checks (format, lint, test)
./qbak-docker.sh pre-commit

# Start development shell
./qbak-docker.sh dev

# Build release binary
./qbak-docker.sh build
```

## Available Commands

| Command | Description |
|---------|-------------|
| `dev` | Interactive development shell |
| `build` | Build release binary |
| `test` | Run all tests |
| `fmt` | Format code |
| `clippy` | Run linter |
| `pre-commit` | Run format + lint + test |
| `clean` | Stop dev container and remove project volumes |

## Usage

### Shell script
```bash
./qbak-docker.sh <command>
```

### Makefile
```bash
make <command>
```

### Direct docker compose
```bash
docker compose run --rm qbak cargo <command>
```

## Features

- Latest stable Rust components (rustfmt, clippy)
- Rust version pinned to project MSRV (`rust-version` in Cargo.toml)
- Persistent Cargo cache for faster builds
- User permission handling via host UID/GID
- Minimal Docker layers

## Notes

- Override Rust version by setting `RUST_VERSION`, e.g. `RUST_VERSION=stable docker compose build`.
