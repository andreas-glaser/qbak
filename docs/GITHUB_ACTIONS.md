# GitHub Actions for qbak

This document describes the comprehensive GitHub Actions setup for the qbak project, providing automated CI/CD, security scanning, documentation checks, and release management.

## Overview

The qbak project uses 5 main GitHub Actions workflows:

| Workflow | Purpose | Triggers | Badge |
|----------|---------|----------|-------|
| **CI** | Main testing and building | Push/PR to main, dev (and feature/* pushes) | [![CI](https://github.com/andreas-glaser/qbak/workflows/CI/badge.svg)](https://github.com/andreas-glaser/qbak/actions/workflows/ci.yml) |
| **Security** | Security auditing | Push/PR to main, dev + weekly | [![Security](https://github.com/andreas-glaser/qbak/workflows/Security/badge.svg)](https://github.com/andreas-glaser/qbak/actions/workflows/security.yml) |
| **Documentation** | Docs building/checking | Push/PR to main, dev | [![Documentation](https://github.com/andreas-glaser/qbak/workflows/Documentation/badge.svg)](https://github.com/andreas-glaser/qbak/actions/workflows/docs.yml) |
| **Release** | Automated releases | Version tags | - |
| **Benchmarks** | Performance testing | Push/PR + manual | - |

## Workflows Details

### 1. CI Workflow (`.github/workflows/ci.yml`)

**Purpose**: Main continuous integration pipeline

**Jobs**:
- **Check**: Code formatting, clippy lints, documentation (fails on fmt/clippy warnings)
- **Test**: Cross-platform testing (Linux, macOS, Windows)
- **Build**: Build binaries for all target platforms
- **MSRV**: Minimum Supported Rust Version check (1.71.0)
- **Integration**: Real CLI testing with file operations
- **Unused-deps**: Check for unused dependencies

**Platforms Tested**:
- Ubuntu Latest (Linux x86_64)
- Windows Latest (Windows x86_64)
- macOS Latest (macOS x86_64)

**Rust Versions**:
- 1.71.0 (MSRV)
- Stable
- Beta (Linux only)

**Build Targets**:
- `x86_64-unknown-linux-gnu`
- `x86_64-unknown-linux-musl`
- `aarch64-unknown-linux-gnu`
- `aarch64-unknown-linux-musl`
- `x86_64-apple-darwin`
- `aarch64-apple-darwin`
- `x86_64-pc-windows-msvc`

### 2. Security Workflow (`.github/workflows/security.yml`)

**Purpose**: Comprehensive security scanning and auditing

**Jobs**:
- **Audit**: Dependency vulnerability scanning with `cargo-audit`
- **Deny**: License and dependency policy enforcement with `cargo-deny`
- **Secrets-scan**: Scan for leaked secrets with TruffleHog
- **License-check**: License compliance verification
- **SAST**: Static analysis with `cargo-geiger`
- **Supply-chain**: Dependency analysis with `cargo-machete`
- **Binary-analysis**: Security analysis of release binaries

**Scheduled**: Runs weekly on Mondays at 9 AM UTC

### 3. Documentation Workflow (`.github/workflows/docs.yml`)

**Purpose**: Documentation quality assurance

**Jobs**:
- **Docs**: Build Rust documentation with strict warnings
- **Markdown**: Markdown syntax and style checking
- **Link-check**: Verify all links in documentation
- **Doc-examples**: Test code examples in documentation
- **Spell-check**: Spell checking with custom dictionary
- **Doc-report**: Generate comprehensive documentation report

### 4. Release Workflow (`.github/workflows/release.yml`)

**Purpose**: Automated release management

**Triggered by**: Git tags matching `v*` (e.g., `v1.0.0`)

**Jobs**:
- **Create-release**: Generate GitHub release with changelog
- **Build-release**: Build optimized binaries for all platforms
- **Publish-crate**: Publish to crates.io (stable releases only)
- **Create-checksums**: Generate SHA256 checksums for all artifacts

**Artifacts Created**:
- `qbak-linux-x86_64.tar.gz`
- `qbak-linux-x86_64-musl.tar.gz`
- `qbak-linux-arm64.tar.gz`
- `qbak-linux-arm64-musl.tar.gz`
- `qbak-macos-x86_64.tar.gz`
- `qbak-macos-arm64.tar.gz`
- `qbak-windows-x86_64.zip`
- `checksums.txt`

### 5. Benchmarks Workflow (`.github/workflows/bench.yml`)

**Purpose**: Performance testing and comparison

**Jobs**:
- **Benchmark**: Performance testing with various file sizes
- **Comparison**: Compare performance with standard tools (cp, rsync)

**Metrics Measured**:
- File backup throughput (MB/s)
- Directory processing speed (files/sec)
- Memory usage
- Startup time
- Binary size analysis

## Repository Configuration

### Dependabot (`.github/dependabot.yml`)

Automated dependency updates:
- **Rust dependencies**: Weekly on Mondays
- **GitHub Actions**: Weekly on Mondays
- **Pull request limit**: 5 per ecosystem
- **Auto-assign**: @andreas-glaser

### Issue Templates

#### Bug Report (`.github/ISSUE_TEMPLATE/bug_report.md`)
Structured template for bug reports including:
- Reproduction steps
- Environment details
- Error output
- File/directory context

#### Feature Request (`.github/ISSUE_TEMPLATE/feature_request.md`)
Template for feature suggestions including:
- Use case description
- Proposed solution
- Implementation considerations
- Compatibility concerns

### Pull Request Template (`.github/pull_request_template.md`)

Comprehensive checklist covering:
- Change description and testing
- Documentation updates
- Security considerations
- Cross-platform compatibility
- Performance impact

## Security Configuration

### Cargo Deny (`deny.toml`)

Dependency policy enforcement:
- **License allowlist**: MIT, Apache-2.0, BSD variants
- **License denylist**: GPL, AGPL, copyleft licenses
- **Vulnerability scanning**: Deny known vulnerabilities
- **Multiple versions**: Warn about duplicate dependencies

**Allowed Licenses**:
- MIT
- Apache-2.0
- Apache-2.0 WITH LLVM-exception
- BSD-2-Clause
- BSD-3-Clause
- ISC
- Unicode-DFS-2016

**Security Policies**:
- Deny unmaintained crates (warn)
- Deny yanked crates
- Deny unknown registries
- Warn about unsound code

## Secrets and Environment Variables

### Required Repository Secrets

For full functionality, set these GitHub repository secrets:

| Secret | Purpose | Required For |
|--------|---------|--------------|
| `CARGO_REGISTRY_TOKEN` | Publishing to crates.io | Release workflow |
| `GITHUB_TOKEN` | Automatic (no setup needed) | All workflows |

### Optional Configuration

| Variable | Purpose | Default |
|----------|---------|---------|
| Dependabot reviewers | PR review assignment | @andreas-glaser |
| Issue assignees | Bug/feature assignment | @andreas-glaser |

## Development Workflow

### For Contributors

1. **Fork** the repository
2. **Create** feature branch
3. **Develop** with local testing (or Docker tools):
   ```bash
   # Docker (preferred for consistency)
   make -C tools/docker pre-commit

   # Or locally
   cargo test -- --test-threads=1
   cargo fmt --check
   cargo clippy -- -D warnings
   ```
4. **Push** to fork (triggers CI on PR)
5. **Submit** pull request using template

### For Maintainers

1. **Review** PR (automated checks must pass)
2. **Merge** PR to dev (active development branch)
3. **Create** release tag when ready:
   ```bash
   git tag v1.0.1
   git push origin v1.0.1
   ```
4. **Monitor** release workflow completion

## Monitoring and Reports

### Artifacts Generated

Each workflow run generates artifacts:

- **CI**: Binary artifacts for all platforms (7 days)
- **Security**: Audit reports, license reports (30 days)
- **Documentation**: Doc reports, spell check results (30 days)
- **Benchmarks**: Performance reports, comparisons (30 days)

### Status Monitoring

Check workflow status via:
- **GitHub Actions tab**: Real-time status
- **README badges**: Quick overview
- **Email notifications**: On failure (if configured)

## Customization

### Adding New Platforms

To add new build targets:

1. Add to matrix in `ci.yml` and `release.yml`
2. Update `deny.toml` target list
3. Test cross-compilation locally
4. Update documentation

### Modifying Security Policies

Edit `deny.toml` to:
- Add/remove allowed licenses
- Ignore specific advisories
- Change policy levels (deny/warn/allow)

### Performance Benchmarks

Add new benchmarks in `bench.yml`:
- Create test files
- Add timing measurements
- Update report generation

## Troubleshooting

### Common Issues

**CI Failures**:
- Check clippy warnings: `cargo clippy -- -D warnings`
- Verify formatting: `cargo fmt --check`
- Run tests locally: `cargo test -- --test-threads=1`

**Security Audit Failures**:
- Review advisory database updates
- Check for new vulnerabilities
- Update dependencies if needed

**Release Failures**:
- Verify version in `Cargo.toml` matches tag
- Check crates.io token validity
- Ensure all platforms build successfully

**Documentation Failures**:
- Fix broken links
- Update outdated examples
- Check spelling with custom dictionary

For additional help, see the [Contributing Guide](CONTRIBUTING.md) or open an issue. 
