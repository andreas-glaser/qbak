# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

[Unreleased]: https://github.com/andreas-glaser/qbak/compare/v1.1.1...HEAD
[1.1.1]: https://github.com/andreas-glaser/qbak/compare/v1.1.0...v1.1.1
[1.1.0]: https://github.com/andreas-glaser/qbak/compare/v1.0.0...v1.1.0
[1.0.0]: https://github.com/andreas-glaser/qbak/releases/tag/v1.0.0
[0.1.0]: https://github.com/andreas-glaser/qbak/releases/tag/v0.1.0 