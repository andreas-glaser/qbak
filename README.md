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

## Features

- **Zero-config** – runs with sensible defaults; no config files required
- **Safe & atomic** – never overwrite existing data; fail loudly on errors
- **Cross-platform** – primary target is Linux; works on macOS and Windows (WSL) too
- **Tiny static binary** – single executable with no dependencies
- **Fast** – efficient file operations with progress indication for large operations

## Installation

### From Source

```bash
git clone https://github.com/andreas-glaser/qbak.git
cd qbak
cargo build --release
sudo cp target/release/qbak /usr/local/bin/
```

### Prerequisites

- Rust 1.71 or later

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
qbak [OPTIONS] <TARGET>...

Arguments:
  <TARGET>...  Files or directories to back up

Options:
  -n, --dry-run    Show what would be backed up without doing it
  -v, --verbose    Show detailed progress information
  -q, --quiet      Suppress all output except errors
  -h, --help       Print help
  -V, --version    Print version
```

### Examples

```bash
# Dry run to see what would be backed up
qbak --dry-run important.txt
# Output: Would create backup: important-20250603T145231-qbak.txt (1.2 KB)

# Verbose output for detailed information
qbak --verbose my-project/
# Output: 
# Processed: my-project/
#   → my-project-20250603T145232-qbak/
#   Files: 42
#   Size: 15.3 MB
#   Duration: 0.12s

# Quiet mode (only errors)
qbak --quiet *.txt
```

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

# Maximum filename length before showing error
max_filename_length = 200

# Show progress for operations with more than N files
progress_threshold = 100
```

## Safety Features

- **Never overwrites existing files** – uses collision counters instead
- **Atomic operations** – temporary files ensure no partial backups
- **Input validation** – rejects dangerous paths and filenames
- **Permission preservation** – maintains original file permissions and timestamps
- **Error recovery** – continues with other files if one fails

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
cargo test
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

## Changelog

See [CHANGELOG.md](CHANGELOG.md) for detailed release notes and version history. 