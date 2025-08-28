# Contributing to qbak

Thank you for your interest in contributing to qbak! This document provides guidelines and information for contributors.

## Quick Start

1. **Fork** the repository on GitHub
2. **Clone** your fork locally
3. **Create** a new branch for your changes
4. **Make** your changes and add tests
5. **Test** your changes thoroughly
6. **Submit** a pull request

## Git Workflow

### Branch Strategy

**Main Branches:**
- **`main`** - Stable release branch, contains only released versions
- **`dev`** - Development branch, active development happens here

**Development Process:**
1. **Fork** the repository
2. **Create feature branch** from `dev`: `git checkout dev && git checkout -b feature/my-feature`
3. **Develop and test** your changes
4. **Submit PR** against the `dev` branch
5. **Maintainers merge** to `dev` after review
6. **Periodic releases** merge `dev` → `main` with version tags

**Branch Usage:**
```bash
# Start new feature from dev
git checkout dev
git pull origin dev
git checkout -b feature/awesome-feature

# Submit PR against dev branch (not main)
# After merge, feature branches are deleted

# Releases: dev → main + tag
git checkout main
git merge dev
git tag v1.1.0
git push origin main v1.1.0
```

## Development Environment

### Prerequisites

- Rust 1.71 or later
- Git
- A Unix-like environment (Linux, macOS, or WSL on Windows)

### Setup

```bash
# Clone your fork
git clone https://github.com/YOUR_USERNAME/qbak.git
cd qbak

# Build the project
cargo build

# Run tests
cargo test -- --test-threads=1

# Run with verbose output to see all tests
cargo test -- --test-threads=1 --nocapture
```

## Code Style

### Rust Style Guidelines

- Follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use `cargo fmt` to format code (enforced by CI)
- Use `cargo clippy` to catch common mistakes (enforced by CI)
- Write clear, self-documenting code with appropriate comments

### Formatting

```bash
# Format code
cargo fmt

# Check formatting without changing files
cargo fmt --check

# Run clippy lints
cargo clippy -- -D warnings
```

## Testing

### Running Tests

```bash
# Run all tests
cargo test -- --test-threads=1

# Run tests with output
cargo test -- --test-threads=1 --nocapture

# Run specific test
cargo test test_name -- --test-threads=1

# Run tests for specific module
cargo test backup:: -- --test-threads=1

# Run tests in release mode
cargo test --release -- --test-threads=1
```

### Writing Tests

- Write unit tests for all new functionality
- Include edge cases and error conditions  
- Test cross-platform behavior when possible
- Use descriptive test names that explain what is being tested

Example test structure:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_descriptive_name() {
        // Arrange
        let input = "test data";
        
        // Act
        let result = function_under_test(input);
        
        // Assert
        assert_eq!(result, expected_value);
    }
}
```

## Types of Contributions

### Bug Reports

Use the [bug report template](../.github/ISSUE_TEMPLATE/bug_report.md) and include:
- Clear reproduction steps
- Expected vs actual behavior
- Environment details (OS, filesystem, file types)
- Complete error output

### Feature Requests

Use the [feature request template](../.github/ISSUE_TEMPLATE/feature_request.md) and consider:
- Does this align with qbak's goal of being a simple backup tool?
- Is this cross-platform compatible?
- What are the security implications?

### Code Contributions

1. **Start with an issue** - discuss the change before implementing
2. **Keep changes focused** - one feature/fix per PR
3. **Add tests** - ensure your changes are well-tested
4. **Update documentation** - README, help text, comments
5. **Follow security principles** - never delete files, validate inputs

## Pull Request Process

### Before Submitting

- [ ] All tests pass locally (`cargo test -- --test-threads=1`)
- [ ] Code is formatted (`cargo fmt --check`)
- [ ] No clippy warnings (`cargo clippy -- -D warnings`)
- [ ] Documentation is updated
- [ ] CHANGELOG.md is updated (for significant changes)

### PR Requirements

- Use the [pull request template](../.github/pull_request_template.md)
- Link to related issues
- Provide clear description of changes
- Include test results
- Update version number if needed (follow semantic versioning)

### Review Process

1. **Automated checks** must pass (CI, security, docs)
2. **Manual review** by maintainers
3. **Testing** on multiple platforms when possible
4. **Documentation review** for user-facing changes

## Security Guidelines

qbak is a backup tool, so security is paramount:

### Core Security Principles

- **Never delete files** - qbak should only create, never remove
- **Validate all inputs** - check paths, filenames, sizes
- **Prevent path traversal** - reject `../` patterns
- **Atomic operations** - use temporary files to prevent corruption
- **Clear error messages** - help users understand problems

### Security Considerations

- Path traversal attacks (`../../../etc/passwd`)
- Filename injection and special characters
- Large file handling and disk space
- Permission preservation and security contexts
- Input validation and bounds checking

## Documentation

### Code Documentation

- Document all public APIs with `///` comments
- Include examples in documentation
- Explain complex algorithms or security considerations
- Use `#[doc(hidden)]` for internal APIs

### User Documentation

- Keep README.md current with features
- Update help text for new options
- Provide clear examples
- Document configuration options

## Performance

### Performance Considerations

- Minimize memory usage for large files
- Use efficient file operations
- Consider startup time impact
- Profile performance-critical paths

### Benchmarking

```bash
# Run benchmarks (if implemented)
cargo bench

# Performance testing with real files
./target/release/qbak large_file.bin
```

## Cross-Platform Compatibility

### Supported Platforms

- **Primary**: Linux (x86_64, aarch64)
- **Secondary**: macOS (x86_64, ARM64)
- **Tertiary**: Windows/WSL

### Platform-Specific Code

```rust
#[cfg(unix)]
fn unix_specific_function() {
    // Unix-specific implementation
}

#[cfg(windows)]
fn windows_specific_function() {
    // Windows-specific implementation
}
```

## Release Process

### Versioning

qbak follows [Semantic Versioning](https://semver.org/):
- **MAJOR**: Breaking changes
- **MINOR**: New features (backward compatible)
- **PATCH**: Bug fixes (backward compatible)

### Release Checklist

1. Update version in `Cargo.toml`
2. Update `CHANGELOG.md`
3. Create git tag: `git tag v1.0.0`
4. Push tag: `git push origin v1.0.0`
5. GitHub Actions will automatically create release

## Communication

### Getting Help

- **GitHub Issues**: Bug reports and feature requests
- **GitHub Discussions**: General questions and ideas
- **Email**: andreas.glaser@pm.me for security issues

### Code of Conduct

- Be respectful and inclusive
- Focus on constructive feedback
- Help others learn and improve
- Follow GitHub's [Community Guidelines](https://docs.github.com/en/github/site-policy/github-community-guidelines)

## Recognition

Contributors will be recognized in:
- Git commit history
- GitHub contributor list
- Release notes (for significant contributions)
- Special thanks in README (for major features)

Thank you for contributing to qbak! 🦀 