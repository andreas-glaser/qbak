# qbak - Quick Backup

[![CI](https://github.com/andreas-glaser/qbak/workflows/CI/badge.svg)](https://github.com/andreas-glaser/qbak/actions/workflows/ci.yml)
[![Security](https://github.com/andreas-glaser/qbak/workflows/Security/badge.svg)](https://github.com/andreas-glaser/qbak/actions/workflows/security.yml)
[![Documentation](https://github.com/andreas-glaser/qbak/workflows/Documentation/badge.svg)](https://github.com/andreas-glaser/qbak/actions/workflows/docs.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Crates.io](https://img.shields.io/crates/v/qbak.svg)](https://crates.io/crates/qbak)

A single-command backup helper for Linux and POSIX systems written in Rust.

## Overview

`qbak` creates timestamped backup copies of files and directories with zero configuration. It's designed for quick, safe backups with sensible defaults.

```bash
qbak example.txt        → example-20250603T145231-qbak.txt
qbak photos/            → photos-20250603T145232-qbak/
```

## Why qbak?

Have you ever found yourself editing important files or directories and wanting to create a quick local backup first? You know the routine: `cp myconfig.conf backup-myconfig.conf` or something similar. But then you realize your backup naming lacks consistency-no timestamps, no predictable convention, just ad-hoc names that become meaningless over time.

That's exactly where `qbak` comes in. It's a super simple tool designed for lightning-fast local backups with a consistent, timestamped naming convention. Nothing more, nothing less. Just the backup utility you wish you'd had all along.

## Features

- **Zero-config** - runs with sensible defaults; no config files required
- **Safe & atomic** - never overwrite existing data; fail loudly on errors
- **Cross-platform** - primary target is Linux; works on macOS and Windows (WSL) too
- **Tiny static binary** - single executable with no dependencies
- **Smart progress indication** - automatic progress bars for large operations with adaptive terminal layouts

## Installation

### From GitHub Releases (Recommended)

Download the latest release from [GitHub Releases](https://github.com/andreas-glaser/qbak/releases):

```bash
# Download the latest release for Linux x86_64
wget https://github.com/andreas-glaser/qbak/releases/latest/download/qbak-linux-x86_64.tar.gz
tar -xzf qbak-linux-x86_64.tar.gz

# Install system-wide (requires sudo)
sudo cp qbak /usr/bin/

# Or install for current user only
mkdir -p ~/.local/bin
cp qbak ~/.local/bin/

# Check if it is installed correctly
qbak --version
```

**Available releases:**
- `qbak-linux-x86_64.tar.gz` - Linux x86_64 (glibc)
- `qbak-linux-x86_64-musl.tar.gz` - Linux x86_64 (musl, static binary)
- `qbak-linux-arm64.tar.gz` - Linux ARM64 (glibc, for Raspberry Pi 4/5, ARM64 servers)
- `qbak-linux-arm64-musl.tar.gz` - Linux ARM64 (musl, static binary)
- `qbak-linux-armv7l.tar.gz` - Linux ARMv7l (glibc, for Raspberry Pi 2/3, ARMv7 devices)
- `qbak-macos-x86_64.tar.gz` - macOS x86_64 (Intel)
- `qbak-macos-arm64.tar.gz` - macOS ARM64 (Apple Silicon)
- `qbak-windows-x86_64.zip` - Windows x86_64

### From Source

```bash
git clone https://github.com/andreas-glaser/qbak.git
cd qbak
cargo build --release
sudo cp target/release/qbak /usr/bin/
```

### Prerequisites

- Rust 1.71 or later (for building from source)

## Usage

### Basic Usage

```bash
# Backup a single file
qbak important.txt

# Backup multiple files
qbak file1.txt file2.txt config.json

# Backup a directory
qbak my-project/

# Backup multiple directories
qbak docs/ src/ tests/
```

### Command Line Options

```bash
qbak [OPTIONS] [TARGET]...

Arguments:
  [TARGET]...      Files or directories to back up

Options:
  -n, --dry-run        Show what would be backed up without doing it
  -v, --verbose        Show detailed progress information
  -q, --quiet          Suppress all output except errors
      --progress       Force progress indication even for small operations
      --no-progress    Disable progress indication
      --dump-config    Display current configuration settings and exit
  -h, --help           Print help
  -V, --version        Print version
```

### Examples

```bash
# Dry run to see what would be backed up
qbak --dry-run important.txt
# Output: Would create backup: important-20250603T145231-qbak.txt (1.2 KB)

# Verbose output for detailed information
qbak --verbose my-project/
# Output: 
Processed: my-project/
  → my-project-20250603T145232-qbak/
  Files: 42
  Size: 15.3 MB
  Duration: 0.12s

# Quiet mode (only errors)
qbak --quiet *.txt

# Force progress indication for small operations
qbak --progress single-file.txt
# Output: Shows progress bar even for small files

# Disable progress indication for large operations
qbak --no-progress large-directory/
# Output: No progress bars, even for operations that normally show them

# Check current configuration
qbak --dump-config
# Output: Shows config file location, all settings, and example backup names
```

## Progress Indication

`qbak` automatically shows progress bars for large backup operations to keep you informed during lengthy transfers.

### Smart Auto-Detection

Progress indication is automatically enabled when operations meet any of these thresholds:
- **≥50 files** to process
- **≥10 MB** total data size  
- Operations taking longer than expected

The progress display adapts to your terminal:
- **Wide terminals (≥120 cols)**: Full progress with file details, transfer rates, and ETA
- **Normal terminals (≥80 cols)**: Compact progress with essential information
- **Narrow terminals**: Minimal progress indication

### Two-Phase Progress

For directory backups, `qbak` shows progress in two phases:

1. **Scanning Phase**: Discovers files and calculates total size
   ```
   ⠋ Scanning files... 127 files found, current: photo.jpg
   ```

2. **Backup Phase**: Copies files with detailed progress
   ```
   [████████████████████████████████] 127/127 files (100%) • 45.2 MB/45.2 MB • 12.3 MB/s • ETA: 0s • Processing: document.pdf
   ```

### Manual Control

- `--progress`: Force progress indication even for small operations
- `--no-progress`: Disable progress indication entirely
- Configuration file settings override auto-detection

### Interactive vs Non-Interactive

Progress bars are only shown in interactive terminals. In CI environments, scripts, or when output is redirected, progress indication is automatically disabled to avoid cluttering logs.

## Naming Scheme

Backup files follow the pattern:
```
<stem>-YYYYMMDDTHHMMSS-qbak[-N].<ext>
```

- **ISO-8601 timestamp** (basic format, sortable, Windows-safe)
- **Original extension preserved** so applications recognize file type
- **Collision counter** (`-1`, `-2`, etc.) if backup with same timestamp exists

Examples:
- `report.pdf` → `report-20250603T145231-qbak.pdf`
- `data.tar.gz` → `data.tar-20250603T145231-qbak.gz`
- `makefile` → `makefile-20250603T145231-qbak`

## Configuration

Optional configuration file: `~/.config/qbak/config.ini`

```ini
[qbak]
# Timestamp format for backup names (ISO-8601 basic format)
timestamp_format = YYYYMMDDTHHMMSS

# Suffix added to backup filenames  
backup_suffix = qbak

# Preserve original file permissions and timestamps (true/false)
preserve_permissions = true

# Follow symbolic links (copy target) or preserve as symlinks
follow_symlinks = true

# Include hidden files when backing up directories  
include_hidden = true

# Maximum filename length before showing error (filesystem limit: 255)
max_filename_length = 255

# Progress indication settings
[progress]
# Enable/disable progress indication (true/false)
enabled = true

# Force progress indication regardless of thresholds (true/false)  
force_enabled = false

# Minimum number of files to show progress
min_files_threshold = 50

# Minimum total size to show progress (in bytes)
min_size_threshold = 10485760  # 10 MB

# Minimum expected duration to show progress (in seconds)
min_duration_threshold = 2
```

## Safety Features

- **Never overwrites existing files** - uses collision counters instead
- **Atomic operations** - temporary files ensure no partial backups
- **Input validation** - rejects dangerous paths and filenames
- **Permission preservation** - maintains original file permissions and timestamps
- **Error recovery** - continues with other files if one fails

## Target Audience

- Linux system administrators
- Server operators  
- DevOps engineers
- Command-line power users
- Automation scripts

## Platform Support

- **Linux** (primary target)
- **macOS** 
- **Windows** (via WSL)

## Development

### Building

```bash
git clone https://github.com/andreas-glaser/qbak.git
cd qbak
cargo build
```

### Testing

```bash
cargo test -- --test-threads=1
```

The project has comprehensive unit tests covering all modules and edge cases.

### Release Build

```bash
cargo build --release
```

This creates an optimized binary at `target/release/qbak` (~849KB).

## License

MIT License - see [LICENSE](LICENSE) file.

## Author

Andreas Glaser <andreas.glaser@pm.me>

## Contributing

Contributions welcome! Please feel free to submit issues and pull requests.

**Development Branch**: Active development happens on the `dev` branch. Please submit pull requests against `dev` rather than `main`. See [CONTRIBUTING.md](CONTRIBUTING.md) for detailed guidelines.

## Changelog

See [CHANGELOG.md](CHANGELOG.md) for detailed release notes and version history. 