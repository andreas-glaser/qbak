# Quick Backup - qbak

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

* **Zero-config** - runs with sensible defaults; no config files required.
* **Safe & atomic** - never overwrite existing data; fail loudly on errors.
* **Cross-platform** - primary target is Linux; should compile and work on macOS
  and Windows (WSL) too.
* **Tiny static binary** - one executable placed on the user's $PATH.
* **Debian/Ubuntu package** - eventual `apt install qbak` via official repos.

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
  -n, --dry-run        Show what would be backed up without doing it
  -v, --verbose        Show detailed progress information  
  -q, --quiet          Suppress all output except errors
      --progress       Force progress indication even for small operations
      --no-progress    Disable progress indication completely
      --dump-config    Display current configuration settings and exit
  -h, --help           Show help information
  -V, --version        Show version information
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
max_filename_length = 255
```

**Cross-platform compatibility:**
* INI format works excellently across all platforms
* Windows users very familiar with INI files
* Linux/Unix admins know INI from Git, SSH, system configs
* Standard configuration directories handled automatically
* File paths normalized for each platform

## Progress Indication System

### Overview

Provide real-time feedback for backup operations that take significant time, enhancing user experience while maintaining qbak's simplicity and safety guarantees.

### When to Show Progress

**Automatic activation thresholds:**
* **File count**: Directory backups with ≥50 files
* **Data size**: Operations involving ≥10 MB of data
* **Time duration**: Any operation taking ≥2 seconds

**Command line control:**
* `--quiet`: Completely disable progress indication
* `--verbose`: Enhanced progress with detailed information
* `--progress`: Force progress indication even below thresholds
* `--no-progress`: Disable progress indication (overrides automatic detection)

**Dynamic detection:**
```bash
# Small operations - no progress shown by default
qbak single-file.txt
# → Created backup: single-file-20250603T145231-qbak.txt (1.2 KB)

# Force progress for small operations
qbak --progress single-file.txt
# → [████████████████████████████████] 1/1 files (100%) • 1.2 KB/1.2 KB • ETA: 0s
# → Created backup: single-file-20250603T145231-qbak.txt (1.2 KB)

# Large operations - automatic progress
qbak large-project/
# → [████████████████████████████████] 1,247/1,250 files (98%) • 45.2 MB/46.1 MB • 12.3 MB/s • ETA: 1s
# → Created backup: large-project-20250603T145232-qbak/ (1,250 files, 46.1 MB)

# Disable progress even for large operations
qbak --no-progress large-project/
# → Created backup: large-project-20250603T145233-qbak/ (1,250 files, 46.1 MB)
```

### Progress Display Levels

**Compact Mode (Default):**
```
[████████████████████████████████] 1,247/1,250 files (98%) • ETA: 2s
```

**Verbose Mode (`--verbose`):**
```
Processing: src/main.rs
[████████████████████████████████] 1,247/1,250 files (98%)
 • Files: 1,247/1,250 (99.8%)
 • Data: 45.2 MB/46.1 MB (98.0%)  
 • Rate: 12.3 MB/s
 • ETA: 2s
 • Current: src/main.rs (15.4 KB)
```

**Dry-run Mode (`--dry-run`):**
```
Scanning: src/main.rs
[████████████████████████████████] 1,247/1,250 files scanned (98%) • ETA: 1s
Would create backup: large-project-20250603T145232-qbak/ (1,250 files, 46.1 MB)
```

### Cross-Platform Implementation

**Terminal Capability Detection:**
```rust
pub struct ProgressConfig {
    pub enabled: bool,
    pub supports_ansi: bool,
    pub terminal_width: usize,
    pub is_interactive: bool,
}

impl ProgressConfig {
    pub fn auto_detect() -> Self {
        Self {
            enabled: !is_ci_environment(),
            supports_ansi: supports_ansi_colors(),
            terminal_width: terminal_width().unwrap_or(80),
            is_interactive: atty::is(atty::Stream::Stdout),
        }
    }
}
```

**Fallback Strategies:**
* **No ANSI support**: Use simple text updates (`Files processed: 1247/1250`)
* **Narrow terminals**: Compress progress display (`1247/1250 (98%)`)
* **Non-interactive**: Periodic status updates instead of live progress
* **CI environments**: Disable progress, use milestone logging

**Platform-Specific Handling:**
```rust
// Windows: Handle console API differences
#[cfg(windows)]
fn setup_progress_display() -> Result<()> {
    // Enable ANSI escape sequences on Windows 10+
    // Fall back to basic text on older Windows
}

// Unix/Linux: Standard ANSI escape sequences
#[cfg(unix)]
fn setup_progress_display() -> Result<()> {
    // Use termios for terminal size detection
    // Handle SIGWINCH for terminal resize
}
```

### Technical Architecture

**Two-Phase Approach:**
```rust
pub struct BackupProgress {
    phase: ProgressPhase,
    files_total: Option<usize>,
    files_processed: usize,
    bytes_total: Option<u64>,
    bytes_processed: u64,
    start_time: Instant,
    current_file: Option<PathBuf>,
}

enum ProgressPhase {
    Scanning,   // Counting files and calculating sizes
    Backing,    // Actually performing backups
}
```

**Phase 1 - Scanning (Fast):**
* Traverse directory structure
* Count files and calculate total size
* Respect include_hidden and follow_symlinks settings
* Build file list for processing
* Show scanning progress: `Scanning: 1,247 files found...`

**Phase 2 - Backup (Slower):**
* Process files from prepared list
* Update progress after each file
* Show transfer rates and ETA
* Handle errors without breaking progress display

**Memory Efficiency:**
```rust
// Stream processing for large directories
pub struct FileScanner {
    // Don't store all paths in memory for huge directories
    total_count: usize,
    total_size: u64,
    // Use iterator-based approach
}
```

### Progress Bar Components

**Visual Elements:**
* **Progress bar**: `[████████████████████████████████]` (32 chars max)
* **Percentage**: `(98%)` - always shown
* **File count**: `1,247/1,250 files` - when scanning complete
* **Data size**: `45.2 MB/46.1 MB` - when size known
* **Transfer rate**: `12.3 MB/s` - rolling average over 5 seconds
* **ETA**: `ETA: 2s` - based on current rate
* **Current file**: Shown in verbose mode only

**Adaptive Layout:**
```rust
fn format_progress_line(width: usize, progress: &BackupProgress) -> String {
    match width {
        w if w >= 120 => format_full_progress(progress),
        w if w >= 80  => format_compact_progress(progress), 
        w if w >= 40  => format_minimal_progress(progress),
        _             => format_tiny_progress(progress),
    }
}
```

### Integration with Existing Features

**Command Line Flags:**
* `--quiet`: Completely disable progress, only show final result/errors
* `--verbose`: Enable detailed progress with current file names and transfer statistics
* `--progress`: Force progress indication even for operations below automatic thresholds
* `--no-progress`: Explicitly disable progress indication (useful for scripting)
* `--dry-run`: Show scanning progress, then summary of what would be backed up

**Flag Precedence (highest to lowest):**
1. `--quiet` - disables all progress
2. `--no-progress` - disables progress but allows other output
3. `--progress` - forces progress even for small operations
4. `--verbose` - enables enhanced progress display
5. Automatic threshold detection

**Error Handling:**
* Progress display must not interfere with error reporting
* Clear progress line before showing errors
* Resume progress after recoverable errors
* Clean shutdown on interruption (Ctrl+C)

**Configuration Options:**
```ini
[progress]
# Enable/disable progress indication (can be overridden by command line flags)
enabled = true

# Minimum thresholds for showing progress (ignored if --progress flag is used)
min_files = 50
min_size_mb = 10  
min_duration_seconds = 2

# Progress display style
style = compact  # compact, verbose, minimal
update_interval_ms = 100

# Terminal handling
auto_detect_capabilities = true
force_ansi = false
max_width = 120

# Force progress indication regardless of thresholds (equivalent to always using --progress)
force_enabled = false
```

### Testing Strategy

**Unit Tests:**
```rust
#[test]
fn test_progress_config_detection() {
    // Test terminal capability detection
}

#[test] 
fn test_progress_formatting() {
    // Test progress line formatting at different widths
}

#[test]
fn test_eta_calculation() {
    // Test ETA estimation accuracy
}
```

**Integration Tests:**
```rust
#[test]
fn test_progress_with_large_directory() {
    // Create directory with 100+ files
    // Verify progress is shown and accurate
}

#[test]
fn test_progress_respects_quiet_flag() {
    // Ensure --quiet disables progress
}
```

**Platform Tests:**
* Test on Windows, Linux, macOS
* Test in CI environments (should auto-disable)
* Test with different terminal emulators
* Test terminal resize handling

**Manual Testing:**
```bash
# Test different scenarios
qbak large-directory/           # Should show progress automatically
qbak --verbose large-directory/ # Should show detailed progress  
qbak --quiet large-directory/   # Should show no progress
qbak --dry-run large-directory/ # Should show scan progress only

# Test new progress flags
qbak --progress single-file.txt  # Should force progress even for small files
qbak --no-progress large-dir/    # Should disable progress even for large operations
qbak --verbose --progress small/ # Should show detailed progress even below thresholds

# Test edge cases
qbak single-file.txt           # Should not show progress (below threshold)
qbak very-large-file.dat       # Should show progress for large files
TERM=dumb qbak large-dir/      # Should fallback gracefully
qbak --progress --quiet file.txt # --quiet should override --progress
```

### Dependencies

**Required Crates:**
* `indicatif = "0.17"` - Cross-platform progress bars and spinners
* `console = "0.15"` - Terminal capability detection and styling
* `atty = "0.2"` - TTY detection for interactive vs non-interactive

**Cargo.toml Configuration:**
```toml
[dependencies]
# ... existing dependencies ...
indicatif = "0.17"
console = "0.15"
atty = "0.2"
```

### Implementation Phases

**Phase 1: Basic Progress Bar**
* File count progress for directory backups
* Simple progress bar with percentage
* Respect --quiet and --verbose flags

**Phase 2: Enhanced Display**  
* Data size progress and transfer rates
* ETA calculation
* Current file display in verbose mode

**Phase 3: Platform Polish**
* Terminal capability detection
* Adaptive display for different widths
* Windows console compatibility

**Phase 4: Configuration**
* Configuration file options
* Threshold customization
* Style preferences

## Testing Requirements

* **Unit tests**: Core functionality (naming, file operations, error handling)
* **Integration tests**: End-to-end CLI behavior, multiple file scenarios
* **Cross-platform tests**: Linux (primary), macOS, Windows WSL
* **Edge case tests**: Large files, special characters, permission scenarios
* **Performance tests**: Directory with 1000+ files, files >1GB
* **Progress tests**: Progress indication accuracy, terminal compatibility

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

# Directory backup with automatic progress
qbak ~/photos/vacation-2024/

# Multiple targets
qbak config.json data.db ~/scripts/

# Dry run to preview
qbak --dry-run ~/large-directory/

# Force progress for small operations
qbak --progress single-file.txt

# Disable progress for scripting
qbak --no-progress ~/large-directory/

# Quiet operation for scripts
qbak --quiet *.log && echo "Logs backed up"
```

## Tech Stack

Language   : Rust (Edition 2021, MSRV 1.71)
Core Crates: clap · chrono · thiserror · configparser · ctrlc · indicatif · console · atty

**Dependency rationale:**
* `clap`: Robust CLI parsing with derive macros
* `chrono`: Reliable timestamp generation
* `thiserror`: Structured error handling with custom error types
* `configparser`: Simple, cross-platform INI file parsing (familiar format for target users)
* `ctrlc`: Graceful signal handling for interruption
* `indicatif`: Progress bars and spinners for backup operations
* `console`: Terminal capability detection and ANSI support
* `atty`: TTY detection for interactive vs non-interactive environments

## Implementation Architecture

**Code Organization:**
```
src/
├── main.rs        // CLI entry point and argument parsing
├── backup.rs      // Core backup logic (files and directories)
├── naming.rs      // Backup filename generation and collision handling
├── config.rs      // Configuration file loading and defaults
├── error.rs       // Error types and error handling
├── progress.rs    // Progress indication and terminal handling
├── utils.rs       // Utility functions and filesystem operations
└── lib.rs         // Library interface (for testing)
```

**Core Function Interfaces:**
```rust
// Main backup operations
pub fn backup_file(source: &Path, config: &Config) -> Result<BackupResult>;
pub fn backup_directory(source: &Path, config: &Config) -> Result<BackupResult>;
pub fn backup_directory_with_progress(source: &Path, config: &Config, progress: &mut BackupProgress) -> Result<BackupResult>;

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

// Progress indication
pub fn create_progress_bar(config: &ProgressConfig) -> Option<BackupProgress>;
pub fn should_show_progress(file_count: usize, total_size: u64, force_progress: bool) -> bool;
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
    pub max_filename_length: usize,
    pub progress: ProgressConfig,
}

#[derive(Debug)]
pub struct BackupResult {
    pub source_path: PathBuf,
    pub backup_path: PathBuf,
    pub files_processed: usize,
    pub total_size: u64,
    pub duration: Duration,
}

// Progress indication structures
#[derive(Debug, Clone)]
pub struct ProgressConfig {
    pub enabled: bool,
    pub force_enabled: bool,
    pub supports_ansi: bool,
    pub terminal_width: usize,
    pub is_interactive: bool,
    pub min_files_threshold: usize,
    pub min_size_threshold: u64,
    pub min_duration_threshold: Duration,
}

pub struct BackupProgress {
    phase: ProgressPhase,
    files_total: Option<usize>,
    files_processed: usize,
    bytes_total: Option<u64>,
    bytes_processed: u64,
    start_time: Instant,
    current_file: Option<PathBuf>,
    progress_bar: Option<indicatif::ProgressBar>,
}

enum ProgressPhase {
    Scanning,   // Counting files and calculating sizes
    Backing,    // Actually performing backups
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
    #[error("Backup already exists: {path}")]
    BackupExists { path: PathBuf },
    #[error("Path traversal attempt detected: {path}")]
    PathTraversal { path: PathBuf },
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Configuration error: {message}")]
    Config { message: String },
    #[error("Operation interrupted by user")]
    Interrupted,
    #[error("Validation error: {message}")]
    Validation { message: String },
}

