# qbak

<!-- Test change for pre-commit hook -->

A fast and reliable backup utility written in Rust.

## Features

- **Fast**: Efficient file copying with atomic operations
- **Safe**: Data integrity checks and collision detection  
- **Flexible**: Configurable backup naming and policies
- **Cross-platform**: Works on Linux, macOS, and Windows

## Installation

```bash
cargo install qbak
```

## Usage

### Basic file backup
```bash
qbak file.txt
# Creates: file-YYYYMMDDTHHMMSS-qbak.txt
```

### Directory backup
```bash
qbak my_project/
# Creates: my_project-YYYYMMDDTHHMMSS-qbak/
```

### Options
```bash
qbak --help
```

## Configuration

qbak uses a configuration file located at:
- Linux: `~/.config/qbak/config.ini`
- macOS: `~/.config/qbak/config.ini`  
- Windows: `%APPDATA%\qbak\config.ini`

Example configuration:
```ini
[qbak]
timestamp_format = YYYYMMDDTHHMMSS
backup_suffix = qbak
preserve_permissions = true
follow_symlinks = true
include_hidden = true
max_filename_length = 255
```

## License

Licensed under either of:
- Apache License, Version 2.0
- MIT License

at your option. 