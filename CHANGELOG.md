# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

### Fixed

### Changed

## [1.4.0] - 2025-01-14

### Added
- **ARM64 Linux Support** - Added native ARM64 Linux builds for better performance on ARM-based systems
  - New release targets: `aarch64-unknown-linux-gnu` and `aarch64-unknown-linux-musl`
  - Cross-compilation support with proper ARM64 GCC toolchain configuration
  - Automatic CI/CD pipeline builds for ARM64 Linux targets
  - New release artifacts:
    - `qbak-linux-arm64.tar.gz` - ARM64 Linux (dynamically linked with glibc)
    - `qbak-linux-arm64-musl.tar.gz` - ARM64 Linux (statically linked with musl)
  - Perfect for Raspberry Pi 4/5, ARM64 servers, and Apple Silicon machines running Linux

### Changed
- **CI/CD Pipeline** - Enhanced GitHub Actions workflows to build and test ARM64 Linux targets
- **Documentation** - Updated README and documentation to include ARM64 Linux installation instructions
- **Release Process** - Expanded release artifacts from 5 to 7 supported platforms

## [1.3.3] - 2025-07-03

### Fixed
- **Progress Display**: Fixed progress bar appearing after CTRL+C interruption
  - Progress bar now immediately clears when interrupt is detected during backup operations
  - Improved signal handler messaging for cleaner interruption experience
  - User now sees clean sequence: interrupt message → error → cleanup confirmation
  - Resolves confusing display where progress bar updated after "Interrupted by user" message

## [1.3.2] - 2025-01-07

### Fixed
- Fixed signal handling test failures in `create_backup_guard` fallback behavior
- Global context now properly initialized when no context exists during backup operations
- Fixed clippy `manual_flatten` warning in test code
- Fixed CI race conditions by adding `--test-threads=1` to all test jobs including MSRV

### Changed
- **Code Quality**: Cleaned up repetitive and verbose comments across codebase
- Removed redundant "(backward compatibility)" suffixes from signal handling documentation
- Simplified overly verbose comments in signal cleanup implementation
- Improved code readability while maintaining all functionality

### Internal
- All 120 tests passing with improved signal handling reliability
- Maintained code quality standards with clean, concise documentation
- Enhanced CI pipeline reliability across all Rust versions

## [1.3.1] - 2025-06-30

### Fixed
- **Critical**: Fixed incomplete backup cleanup on interruption (CTRL+C)
  - Partial backup files/directories are now properly removed when operations are interrupted
  - Implements global operation tracking with RAII-based cleanup guards
  - Signal handler now cleanups all active backup operations on CTRL+C
  - Comprehensive test coverage for interruption scenarios and cleanup behavior
  - Resolves user expectation that "Cleaning up..." message should actually clean up partial backups
- Fixed clippy warning for unnecessary borrow in file write operation
- Fixed CI race conditions in signal handling tests by using single-threaded test execution
- Improved development workflow by ignoring IDE-specific configuration files

### Internal
- Enhanced CI pipeline reliability with `--test-threads=1` for consistent test execution
- Updated git hooks to prevent race conditions in local development
- Improved development environment setup by excluding `.cursor/` directory from version control

## [1.3.0] - 2025-06-30

### Added
- **Advanced Progress Indication System** - Smart, adaptive progress bars for backup operations
  - Auto-detection: Shows progress for operations with ≥50 files, ≥10 MB data, or long duration
  - Two-phase progress: Scanning phase (file discovery) + Backup phase (actual copying)  
  - Adaptive display: Adjusts to terminal width and capabilities
  - Interactive vs non-interactive detection (disabled in CI environments)
  - New command line flags:
    - `--progress`: Force progress indication even for small operations
    - `--no-progress`: Disable progress indication completely
  - Configuration file support for progress thresholds and behavior
  - Cross-platform terminal capability detection and ANSI support
  - Comprehensive unit test coverage for progress display methods

### Fixed
- **Security**: Replaced deprecated `atty` crate with `std::io::IsTerminal` (RUSTSEC-2024-0375)
- Fixed all remaining clippy warnings for uninlined format arguments
- Improved MSRV (Minimum Supported Rust Version) compatibility for progress features
- Enhanced CI pipeline to include feature branch testing and code formatting
- Corrected verbose output examples in documentation

### Changed
- Enhanced documentation with comprehensive progress indication examples
- Improved code formatting consistency across the project

## [1.2.0] - 2025-06-28

### Added
- Enhanced test coverage analysis and reporting (86 comprehensive tests)
- Improved development workflow with pre-commit hooks and quality checks

### Fixed
- Comprehensive clippy warning resolution across entire codebase
- Fixed all `uninlined_format_args` warnings for better code consistency
- Fixed `format!` macro usage in error handling and tests
- Maintained Rust 1.71 MSRV compatibility with dependency constraints

### Changed
- Improved code quality and formatting consistency throughout project
- Streamlined development environment configuration
- Enhanced documentation structure and completeness
- Strengthened CI/CD pipeline with better quality gates

### Internal
- Consolidated multiple patch releases (1.1.2, 1.1.3) into this minor release
- Improved development branch management and release workflow
- Enhanced pre-commit validation and testing procedures

## [1.1.3] - 2025-06-28

### Fixed
- Fixed dependency version constraints to maintain Rust 1.71 MSRV compatibility
- Constrained clap to <4.5 to avoid requiring Rust 1.74+

## [1.1.2] - 2025-06-28

### Fixed
- Fixed all clippy warnings related to uninlined format arguments
- Improved code quality and formatting consistency
- Fixed format! macro usage in error handling tests

## [1.1.1] - 2025-06-07

### Fixed
- Fixed crates.io publishing issue (no functional changes from 1.1.0)

## [1.1.0] - 2025-06-07 (Unpublished)

### Added
- `--dump-config` flag to display current configuration settings and their source
  - Shows config file path and whether it exists
  - Displays all current settings with their values
  - Provides examples of backup names with current settings
  - Includes instructions for creating config file if none exists

### Fixed
- Progress indication now works as intended for directory backups
  - Only shows when `--verbose` flag is used (respects user intent)
  - Improved messaging: "Backing up directory with X files..." (clearer wording)
  - Displays progress dots during backup (every 10th file)
  - Shows "Directory backup completed: X files processed" at end

### Changed
- Progress indication now always shows when `--verbose` is used (removed `progress_threshold` config entirely)
- Increased `max_filename_length` from 200 to 255 characters to match standard filesystem limits

## [1.0.0] - 2025-01-17

### Added
- Initial stable release of qbak backup utility
- Core backup functionality for files and directories
- Timestamped backup naming with collision resolution (`-1`, `-2`, etc.)
- Cross-platform support (Linux, macOS, Windows/WSL)
- Configuration file support (`~/.config/qbak/config.ini`)
- Command-line interface with multiple options:
  - `--dry-run` - Preview operations without executing
  - `--verbose` - Detailed progress information
  - `--quiet` - Suppress all output except errors
- Comprehensive error handling with helpful suggestions
- Safety features:
  - Never overwrites existing files
  - Atomic operations using temporary files
  - Input validation and path traversal protection
  - Permission and timestamp preservation
- Full unit test coverage (81 tests)
- MIT license
- Complete documentation (README, help text, examples)

### Security
- Path traversal protection (rejects `../` patterns)
- Filename validation (length limits, invalid characters)
- Input sanitization for all user-provided paths
- Atomic file operations prevent corruption
- Signal handling for graceful cleanup on interruption

## [0.1.0] - 2025-06-03

### Added
- Initial release of qbak backup utility
- Core backup functionality for files and directories
- Timestamped backup naming with collision resolution (`-1`, `-2`, etc.)
- Cross-platform support (Linux, macOS, Windows/WSL)
- Configuration file support (`~/.config/qbak/config.ini`)
- Command-line interface with multiple options:
  - `--dry-run` - Preview operations without executing
  - `--verbose` - Detailed progress information
  - `--quiet` - Suppress all output except errors
- Comprehensive error handling with helpful suggestions
- Safety features:
  - Never overwrites existing files
  - Atomic operations using temporary files
  - Input validation and path traversal protection
  - Permission and timestamp preservation
- Full unit test coverage (81 tests)
- MIT license
- Complete documentation (README, help text, examples)

### Security
- Path traversal protection (rejects `../` patterns)
- Filename validation (length limits, invalid characters)
- Input sanitization for all user-provided paths
- Atomic file operations prevent corruption
- Signal handling for graceful cleanup on interruption

[Unreleased]: https://github.com/andreas-glaser/qbak/compare/v1.4.0...HEAD
[1.4.0]: https://github.com/andreas-glaser/qbak/compare/v1.3.3...v1.4.0
[1.3.3]: https://github.com/andreas-glaser/qbak/compare/v1.3.2...v1.3.3
[1.3.2]: https://github.com/andreas-glaser/qbak/compare/v1.3.1...v1.3.2
[1.3.1]: https://github.com/andreas-glaser/qbak/compare/v1.3.0...v1.3.1
[1.3.0]: https://github.com/andreas-glaser/qbak/compare/v1.2.0...v1.3.0
[1.2.0]: https://github.com/andreas-glaser/qbak/compare/v1.1.3...v1.2.0
[1.1.3]: https://github.com/andreas-glaser/qbak/compare/v1.1.2...v1.1.3
[1.1.2]: https://github.com/andreas-glaser/qbak/compare/v1.1.1...v1.1.2
[1.1.1]: https://github.com/andreas-glaser/qbak/compare/v1.1.0...v1.1.1
[1.1.0]: https://github.com/andreas-glaser/qbak/compare/v1.0.0...v1.1.0
[1.0.0]: https://github.com/andreas-glaser/qbak/releases/tag/v1.0.0
[0.1.0]: https://github.com/andreas-glaser/qbak/releases/tag/v0.1.0 