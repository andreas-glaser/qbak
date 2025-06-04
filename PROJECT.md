# Quick Backup — qbak

Version: 1.0  (2025-06-03)

## Summary

qbak is a single-command backup helper for Linux (and other POSIX systems).
Given one or more files or directories, it creates sibling copies tagged with
an ISO-8601 timestamp plus the marker "qbak", and auto-increments if a name
collision occurs.

```
qbak example.txt        → example-20250603T145231-qbak.txt
qbak photos/            → photos-20250603T145232-qbak/
```

## Project Goals

* **Zero-config** – runs with sensible defaults; no config files required.
* **Safe & atomic** – never overwrite existing data; fail loudly on errors.
* **Cross-platform** – primary target is Linux; should compile and work on macOS
  and Windows (WSL) too.
* **Tiny static binary** – one executable placed on the user's $PATH.
* **Debian/Ubuntu package** – eventual `apt install qbak` via official repos.

## Security Principles

**Core Security Requirements:**
* **No destructive operations** - NEVER delete, overwrite, or modify existing files
* **Input validation** - Sanitize all file paths and reject dangerous patterns
* **Path traversal protection** - Prevent `../` attacks and absolute path manipulation
* **Privilege separation** - Run with minimal required permissions
* **Secure temporary files** - Use OS-provided secure temporary directories with proper permissions

**Security Boundaries:**
* **Read-only source access** - Only read source files, never modify them
* **Write-only target creation** - Only create new backup files, never modify existing ones
* **Fail-safe defaults** - On any ambiguity or error, choose the safer option
* **No external dependencies** - Minimize attack surface by avoiding network/external calls
* **Permission preservation** - Maintain but never escalate file permissions

**Threat Model:**
* **Malicious filenames** - Handle Unicode, control characters, extreme lengths safely
* **Race conditions** - Atomic operations prevent time-of-check/time-of-use bugs
* **Disk exhaustion** - Pre-check available space; fail gracefully on insufficient storage
* **Signal interruption** - Clean up safely on SIGINT/SIGTERM without leaving corruption

## Data Safety Guarantees

**Absolute Safety Requirements:**
* **NEVER delete source files** - Source files remain untouched under all circumstances
* **NEVER overwrite existing files** - Auto-increment backup names to prevent collisions
* **NEVER partially corrupt** - Either complete operation succeeds or fails cleanly
* **NEVER leave partial backups** - Clean up incomplete operations on failure/interruption

**Collision Handling:**
```bash
# Automatic collision resolution - never overwrites
qbak example.txt → example-20250603T145231-qbak.txt
qbak example.txt → example-20250603T145231-qbak-1.txt  # Same second
qbak example.txt → example-20250603T145231-qbak-2.txt  # Same second again
```

**Atomic Operations:**
* **Directory creation** - Create backup directory structure atomically
* **File copying** - Copy to temporary name, then rename (atomic on POSIX)
* **Metadata preservation** - Set permissions/timestamps after successful copy
* **Cleanup guarantee** - Remove temporary files on any failure

**Error Recovery:**
* **Pre-flight checks** - Validate inputs and resources before starting
* **Transactional approach** - Track all operations for rollback capability
* **Signal handling** - Graceful cleanup on interruption
* **Resource monitoring** - Monitor disk space during operation

## Standards Compliance

**POSIX Compliance:**
* **File operations** - Use standard POSIX file system calls
* **Path handling** - Follow POSIX path conventions and limits
* **Signal handling** - Proper SIGINT/SIGTERM handling per POSIX
* **Error codes** - Standard exit codes (0=success, 1=general error, 2=misuse)
* **Character encoding** - UTF-8 support with proper normalization

**Cross-Platform Standards:**
* **File naming** - Avoid problematic characters (`<>:"|?*` on Windows)
* **Path separators** - Handle `/` and `\` appropriately per platform
* **Reserved names** - Avoid Windows reserved names (CON, PRN, AUX, etc.)
* **Case sensitivity** - Handle case-insensitive filesystems correctly
* **Line endings** - Preserve original line endings in text files

**Configuration Standards:**
* **XDG Base Directory** - Linux/Unix: `$XDG_CONFIG_HOME` or `~/.config/`
* **Windows Standards** - Windows: `%APPDATA%` for user configuration
* **INI Format** - RFC-compliant INI parsing with proper escaping
* **UTF-8 encoding** - All configuration files use UTF-8 encoding

**Timestamp Standards:**
* **ISO-8601 Basic Format** - `YYYYMMDDTHHMMSS` (sortable, filesystem-safe)
* **UTC normalization** - All timestamps in UTC to avoid timezone issues
* **Leap second handling** - Proper handling of leap seconds in timestamps
* **Y2038 compliance** - Use 64-bit timestamps for future compatibility

**Security Standards:**
* **Principle of least privilege** - Request minimal required permissions
* **Secure by default** - Conservative defaults prioritizing safety
* **Input sanitization** - Validate all user inputs against injection attacks
* **Error information** - Don't leak sensitive information in error messages

## Target Audience

**Primary Users:**
* **Linux system administrators** - backing up config files, logs, databases
* **Server operators** - quick backups before system changes or updates  
* **DevOps engineers** - backing up deployment artifacts, configuration
* **Command-line power users** - general file/directory backup needs
* **Automation scripts** - reliable backup operations in shell scripts

**User Experience Level:**
* Comfortable with command-line interfaces
* Familiar with standard Linux/Unix tools (`cp`, `rsync`, `tar`)
* Experienced with INI-style configuration files (`.gitconfig`, `.ssh/config`)
* May not be developers - focus on simplicity over advanced features

**Use Cases:**
* Pre-deployment backups: `qbak /etc/nginx/nginx.conf`
* Database dumps: `qbak /var/backups/mysql-dump.sql`
* Script automation: `qbak --quiet /home/user/important-files/`
* Development workflows: `qbak src/ before major refactoring`

## Command-Line Interface

```
qbak [OPTIONS] <TARGET>...

ARGUMENTS:
  <TARGET>...    Files or directories to back up

OPTIONS:
  -n, --dry-run     Show what would be backed up without doing it
  -v, --verbose     Show detailed progress information  
  -q, --quiet       Suppress all output except errors
  -h, --help        Show help information
  -V, --version     Show version information
```

**Exit Codes (POSIX Standard):**
* `0` - Success: All backups completed successfully
* `1` - General error: Partial failure or system error
* `2` - Usage error: Invalid arguments or options
* `130` - Interrupted: Operation cancelled by user (Ctrl+C)

## Naming Scheme (Default)

<stem>-YYYYMMDDTHHMMSS-qbak\[-N].<ext>

* Uses ISO-8601 basic format (sortable; `:` avoided for Windows).
* Real extension stays last; applications still recognise file type.
* `-N` counter (-- 1, 2, …) added if a backup with the same timestamp already exists.

**Safety Considerations:**
* **Filename preservation** - Never modify or sanitize source filenames; preserve exactly as-is
* **Length validation** - Fail with clear error if filename would exceed filesystem limits
* **Unicode support** - Handle Unicode filenames properly without modification
* **Case preservation** - Maintain original case on case-sensitive filesystems

## Behavior Details

* **File handling**: Preserves file permissions, ownership (where possible), and timestamps
* **Symbolic links**: Follows symlinks by default (backs up target content)
* **Hidden files**: Includes hidden files (starting with `.`) in directory backups
* **Multiple targets**: Processes targets in sequence; continues on individual failures
* **Empty directories**: Creates backup of empty directories
* **Atomic operations**: Either completes fully or fails cleanly (no partial backups left)

**Security Enhancements:**
* **Path validation** - Reject paths containing `../` or other traversal attempts
* **Symlink loop detection** - Prevent infinite loops from cyclic symlinks
* **Permission checking** - Verify read permissions before attempting operations
* **Resource limits** - Respect system resource limits and quotas

## Error Handling Strategy

* **Insufficient disk space**: Pre-check available space; fail before starting if inadequate
* **Permission denied**: Skip inaccessible files with warning; continue with others
* **Network/special filesystems**: Handle gracefully with appropriate error messages
* **Interrupted operations**: Clean up any partial backups on Ctrl+C or kill signals
* **Invalid targets**: Show clear error messages for non-existent or invalid paths
* **Large operations**: Provide progress indication for operations taking >2 seconds
* **Filename length exceeded**: Fail with helpful error showing maximum length and suggestions
* **Invalid filesystem characters**: Fail with clear explanation of problematic characters and workarounds

**Example Error Messages:**
```bash
# Filename too long
qbak very-long-filename-that-exceeds-limits.txt
Error: Backup filename would exceed filesystem limit (255 characters)
  Source: very-long-filename-that-exceeds-limits.txt (45 chars)
  Backup: very-long-filename-that-exceeds-limits-20250603T145231-qbak.txt (278 chars)
  
Suggestions:
  - Rename the source file to be shorter
  - Move to a directory with a shorter path
  - Use a shorter backup suffix in config

# Invalid characters (Windows)
qbak "file<with>invalid:chars.txt"
Error: Filename contains characters not supported on this filesystem
  Problematic characters: < > :
  
Suggestions:  
  - Rename file to remove: < > : " | ? * characters
  - Use a different filesystem that supports these characters
```

## Edge Cases & Validation

* **Files without extensions**: Backup as `filename-TIMESTAMP-qbak` (no extra dot)
* **Filename length limits**: Show clear error if backup filename would exceed filesystem limits; let user rename source file
* **Files with existing timestamps**: Back up normally; collision counter prevents conflicts
* **System/root files**: No special restrictions; rely on filesystem permissions
* **Special characters**: Preserve Unicode and special characters exactly; handle encoding properly
* **Empty targets**: `qbak` with no arguments shows usage help
* **Invalid characters**: On filesystems that reject certain characters, show clear error with suggestions

## Output & Verbosity

**Default output:**
```bash
qbak example.txt
# → Created backup: example-20250603T145231-qbak.txt

qbak photos/
# → Created backup: photos-20250603T145232-qbak/ (127 files, 45.2 MB)
```

**Quiet mode (`--quiet`):**
```bash
qbak --quiet file.txt
# → (no output on success, errors still shown)
```

**Verbose mode (`--verbose`):**
```bash
qbak --verbose photos/
# → Scanning photos/... (127 files, 45.2 MB)
# → Creating backup: photos-20250603T145232-qbak/
# → [████████████████████████████████] 127/127 files
# → Backup completed: photos-20250603T145232-qbak/
```

**Dry-run mode (`--dry-run`):**
```bash
qbak --dry-run photos/
# → Would create backup: photos-20250603T145232-qbak/ (127 files, 45.2 MB)
```

## Performance Considerations

* **Memory usage**: Stream large files; avoid loading entire contents into memory
* **Progress indication**: Show progress for directories with >100 files or operations >2s
* **Parallel processing**: Sequential processing for simplicity and safety
* **Large files**: Handle files up to available disk space; no arbitrary size limits
* **Checksums**: No integrity verification in v1.0 (keep it simple)

## Limitations & Scope

* **Local filesystem only**: No network, cloud, or remote backup support
* **No compression**: Creates exact copies, no space-saving features
* **No incremental backups**: Each backup is independent and complete
* **No restoration features**: Simple copy-based backups; manual restoration
* **No backup management**: No cleanup, rotation, or organization features
* **Platform-specific features**: Minimal use of OS-specific APIs for maximum portability

## Configuration (Future Consideration)

While maintaining "zero-config" philosophy, power users might eventually benefit from optional configuration:

**Configuration file locations (platform-specific):**
* Linux/macOS: `~/.config/qbak/config.ini`
* Windows: `%APPDATA%\qbak\config.ini`

**Configuration format (INI - familiar to sysadmins):**
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

# Maximum filename length before truncation
max_filename_length = 200

# Show progress for operations with more than N files
progress_threshold = 100
```

**Cross-platform compatibility:**
* INI format works excellently across all platforms
* Windows users very familiar with INI files
* Linux/Unix admins know INI from Git, SSH, system configs
* Standard configuration directories handled automatically
* File paths normalized for each platform

## Testing Requirements

* **Unit tests**: Core functionality (naming, file operations, error handling)
* **Integration tests**: End-to-end CLI behavior, multiple file scenarios
* **Cross-platform tests**: Linux (primary), macOS, Windows WSL
* **Edge case tests**: Large files, special characters, permission scenarios
* **Performance tests**: Directory with 1000+ files, files >1GB

## Distribution & Installation

**Target platforms:**
* Linux (x86_64, arm64) - primary
* macOS (Intel, Apple Silicon)
* Windows (via WSL)

**Binary optimization:**
* Static linking where possible
* Strip debug symbols for release
* LTO (Link Time Optimization) enabled
* Target size: <5MB executable

**Package metadata:**
* License: MIT
* Categories: command-line-utilities, filesystem
* Keywords: backup, copy, timestamp, cli
* Homepage: GitHub repository
* Minimum supported Rust version (MSRV): 1.71

## Documentation Requirements

**Essential documentation:**
* `README.md`: Installation, basic usage, examples
* `CHANGELOG.md`: Version history and changes
* Man page: `qbak(1)` for system integration
* `--help` output: Comprehensive CLI usage
* Error messages: Clear, actionable guidance

**Usage examples to include:**
```bash
# Basic file backup
qbak important-document.pdf

# Directory backup
qbak ~/photos/vacation-2024/

# Multiple targets
qbak config.json data.db ~/scripts/

# Dry run to preview
qbak --dry-run ~/large-directory/

# Quiet operation for scripts
qbak --quiet *.log && echo "Logs backed up"
```

## Tech Stack

Language   : Rust (Edition 2021, MSRV 1.71)
Core Crates: clap · chrono · anyhow · ini · fs_extra (optional) · indicatif (feature)

**Dependency rationale:**
* `clap`: Robust CLI parsing with derive macros
* `chrono`: Reliable timestamp generation
* `anyhow`: Ergonomic error handling
* `ini`: Simple, cross-platform INI file parsing (familiar format for target users)
* `fs_extra`: Enhanced file operations (if needed)
* `indicatif`: Progress bars for large operations

## Implementation Architecture

**Code Organization:**
```
src/
├── main.rs        // CLI entry point and argument parsing
├── backup.rs      // Core backup logic (files and directories)
├── naming.rs      // Backup filename generation and collision handling
├── config.rs      // Configuration file loading and defaults
├── error.rs       // Error types and error handling
├── utils.rs       // Utility functions and filesystem operations
└── lib.rs         // Library interface (for testing)
```

**Core Function Interfaces:**
```rust
// Main backup operations
pub fn backup_file(source: &Path, config: &Config) -> Result<BackupResult>;
pub fn backup_directory(source: &Path, config: &Config) -> Result<BackupResult>;

// Filename generation
pub fn generate_backup_name(source: &Path, config: &Config) -> Result<PathBuf>;
pub fn resolve_collision(base_path: &Path) -> Result<PathBuf>;

// Configuration management
pub fn load_config() -> Result<Config>;
pub fn default_config() -> Config;

// Pre-flight checks
pub fn validate_source(path: &Path) -> Result<()>;
pub fn check_available_space(source: &Path, target_dir: &Path) -> Result<()>;
pub fn validate_backup_filename(path: &Path) -> Result<()>;
```

**Key Data Structures:**
```rust
#[derive(Debug, Clone)]
pub struct Config {
    pub timestamp_format: String,
    pub backup_suffix: String,
    pub preserve_permissions: bool,
    pub follow_symlinks: bool,
    pub include_hidden: bool,
    pub progress_threshold: usize,
}

#[derive(Debug)]
pub struct BackupResult {
    pub source_path: PathBuf,
    pub backup_path: PathBuf,
    pub files_processed: usize,
    pub total_size: u64,
    pub duration: Duration,
}

#[derive(Debug, thiserror::Error)]
pub enum QbakError {
    #[error("Source file not found: {path}")]
    SourceNotFound { path: PathBuf },
    #[error("Backup filename too long: {length} chars (max: {max})")]
    FilenameTooLong { length: usize, max: usize },
    #[error("Insufficient disk space: need {needed} bytes, have {available}")]
    InsufficientSpace { needed: u64, available: u64 },
    #[error("Permission denied: {path}")]
    PermissionDenied { path: PathBuf },
    #[error("Invalid filesystem characters: {chars}")]
    InvalidFilesystemChars { chars: String },
    #[error("Symlink loop detected: {path}")]
    SymlinkLoop { path: PathBuf },
}

