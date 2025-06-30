// Unused imports removed
use indicatif::{ProgressBar, ProgressStyle};
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

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

impl Default for ProgressConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            force_enabled: false,
            supports_ansi: console::colors_enabled(),
            terminal_width: console::Term::stdout().size().1 as usize,
            is_interactive: std::io::stdout().is_terminal(),
            min_files_threshold: 50,
            min_size_threshold: 10 * 1024 * 1024, // 10 MB
            min_duration_threshold: Duration::from_secs(2),
        }
    }
}

impl ProgressConfig {
    pub fn auto_detect() -> Self {
        // Disable in CI environments
        if is_ci_environment() {
            Self {
                enabled: false,
                is_interactive: false,
                ..Self::default()
            }
        } else {
            Self::default()
        }
    }

    pub fn should_show_progress(
        &self,
        file_count: usize,
        total_size: u64,
        force_progress: bool,
    ) -> bool {
        if !self.enabled {
            return false;
        }

        if force_progress {
            return true;
        }

        if self.force_enabled {
            return true;
        }

        // Check thresholds - should show if thresholds are met
        file_count >= self.min_files_threshold || total_size >= self.min_size_threshold
    }
}

pub struct BackupProgress {
    phase: ProgressPhase,
    files_total: Option<usize>,
    files_processed: usize,
    bytes_total: Option<u64>,
    bytes_processed: u64,
    start_time: Instant,
    current_file: Option<PathBuf>,
    progress_bar: Option<ProgressBar>,
    config: ProgressConfig,
}

#[derive(Debug, Clone, Copy)]
enum ProgressPhase {
    Scanning,
    Backing,
}

impl BackupProgress {
    pub fn new(config: ProgressConfig) -> Self {
        Self {
            phase: ProgressPhase::Scanning,
            files_total: None,
            files_processed: 0,
            bytes_total: None,
            bytes_processed: 0,
            start_time: Instant::now(),
            current_file: None,
            progress_bar: None,
            config,
        }
    }

    pub fn start_scanning(&mut self) {
        self.phase = ProgressPhase::Scanning;
        self.start_time = Instant::now();

        if self.config.is_interactive {
            let pb = ProgressBar::new_spinner();
            pb.set_style(
                ProgressStyle::default_spinner()
                    .template("{spinner:.green} Scanning files... {msg}")
                    .unwrap(),
            );
            pb.set_message("Starting scan...");
            self.progress_bar = Some(pb);
        }
    }

    pub fn update_scan_progress(&mut self, files_found: usize, current_path: &Path) {
        self.files_processed = files_found;
        self.current_file = Some(current_path.to_path_buf());

        if let Some(ref pb) = self.progress_bar {
            let filename = current_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("...");
            pb.set_message(format!(
                "Scanning: {files_found} files found, current: {filename}"
            ));
            pb.tick();
        }
    }

    pub fn finish_scanning(&mut self, total_files: usize, total_size: u64) {
        self.files_total = Some(total_files);
        self.bytes_total = Some(total_size);
        self.phase = ProgressPhase::Backing;
        self.files_processed = 0;
        self.bytes_processed = 0;

        // Finish scanning spinner
        if let Some(pb) = self.progress_bar.take() {
            let size_str = format_size(total_size);
            pb.finish_with_message(format!("Scan complete: {total_files} files, {size_str}"));
        }

        // Start backup progress bar
        if self.config.is_interactive && total_files > 0 {
            let pb = ProgressBar::new(total_files as u64);
            pb.set_style(self.get_progress_style());
            self.progress_bar = Some(pb);
        }
    }

    pub fn update_backup_progress(
        &mut self,
        files_completed: usize,
        bytes_completed: u64,
        current_file: &Path,
    ) {
        self.files_processed = files_completed;
        self.bytes_processed = bytes_completed;
        self.current_file = Some(current_file.to_path_buf());

        if let Some(ref pb) = self.progress_bar {
            pb.set_position(files_completed as u64);

            let message = self.format_progress_message(current_file);
            pb.set_message(message);
        }
    }

    pub fn finish(&mut self) {
        if let Some(pb) = self.progress_bar.take() {
            pb.finish_and_clear();
        }
    }

    fn get_progress_style(&self) -> ProgressStyle {
        let template = if self.config.terminal_width >= 120 {
            // Full display for wide terminals
            "[{bar:32.cyan/blue}] {pos}/{len} files ({percent}%) • {bytes}/{total_bytes} • {bytes_per_sec} • ETA: {eta} • {msg}"
        } else if self.config.terminal_width >= 80 {
            // Compact display for normal terminals
            "[{bar:24.cyan/blue}] {pos}/{len} ({percent}%) • {bytes_per_sec} • ETA: {eta}"
        } else if self.config.terminal_width >= 60 {
            // Minimal display for narrow terminals
            "[{bar:16}] {pos}/{len} ({percent}%)"
        } else {
            // Very minimal for very narrow terminals
            "{pos}/{len} ({percent}%)"
        };

        ProgressStyle::default_bar()
            .template(template)
            .unwrap()
            .progress_chars("█▉▊▋▌▍▎▏ ")
    }

    fn format_progress_message(&self, current_file: &Path) -> String {
        let filename = current_file
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("...");

        if self.config.terminal_width >= 120 {
            format!("Processing: {filename}")
        } else {
            // For narrower terminals, truncate long filenames
            let max_len = (self.config.terminal_width / 3).min(30);
            if filename.len() > max_len {
                let truncated = &filename[..max_len.saturating_sub(3)];
                format!("{truncated}...")
            } else {
                filename.to_string()
            }
        }
    }
}

pub fn create_progress_bar(
    config: &ProgressConfig,
    file_count: usize,
    total_size: u64,
    force_progress: bool,
) -> Option<BackupProgress> {
    if config.should_show_progress(file_count, total_size, force_progress) {
        Some(BackupProgress::new(config.clone()))
    } else {
        None
    }
}

pub fn should_show_progress(
    config: &ProgressConfig,
    file_count: usize,
    total_size: u64,
    force_progress: bool,
) -> bool {
    config.should_show_progress(file_count, total_size, force_progress)
}

/// Check if we're running in a CI environment
fn is_ci_environment() -> bool {
    std::env::var("CI").is_ok()
        || std::env::var("GITHUB_ACTIONS").is_ok()
        || std::env::var("GITLAB_CI").is_ok()
        || std::env::var("TRAVIS").is_ok()
        || std::env::var("CIRCLECI").is_ok()
        || std::env::var("JENKINS_URL").is_ok()
        || std::env::var("BUILDKITE").is_ok()
}

/// Format byte size in human-readable format
pub fn format_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    const THRESHOLD: u64 = 1024;

    if bytes < THRESHOLD {
        return format!("{bytes} B");
    }

    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= THRESHOLD as f64 && unit_index < UNITS.len() - 1 {
        size /= THRESHOLD as f64;
        unit_index += 1;
    }

    let unit = UNITS[unit_index];
    format!("{size:.1} {unit}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_config_default() {
        let config = ProgressConfig::default();
        assert!(config.enabled);
        assert!(!config.force_enabled);
        assert_eq!(config.min_files_threshold, 50);
        assert_eq!(config.min_size_threshold, 10 * 1024 * 1024);
        assert_eq!(config.min_duration_threshold, Duration::from_secs(2));
    }

    #[test]
    fn test_progress_config_auto_detect() {
        let config = ProgressConfig::auto_detect();
        assert!(config.enabled || is_ci_environment());
    }

    #[test]
    fn test_should_show_progress_thresholds() {
        let config = ProgressConfig::default();

        // Below thresholds - should not show
        assert!(!config.should_show_progress(10, 1024, false));

        // Above file threshold - should show
        assert!(config.should_show_progress(100, 1024, false));

        // Above size threshold - should show
        assert!(config.should_show_progress(10, 20 * 1024 * 1024, false));

        // Force enabled - should show even below thresholds
        assert!(config.should_show_progress(1, 100, true));
    }

    #[test]
    fn test_should_show_progress_disabled() {
        let config = ProgressConfig {
            enabled: false,
            ..ProgressConfig::default()
        };

        // Should never show when disabled
        assert!(!config.should_show_progress(1000, 1024 * 1024 * 1024, true));
    }

    #[test]
    fn test_backup_progress_creation() {
        let config = ProgressConfig::default();
        let progress = BackupProgress::new(config);

        assert!(matches!(progress.phase, ProgressPhase::Scanning));
        assert_eq!(progress.files_processed, 0);
        assert_eq!(progress.bytes_processed, 0);
        assert!(progress.files_total.is_none());
        assert!(progress.bytes_total.is_none());
    }

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(512), "512 B");
        assert_eq!(format_size(1024), "1.0 KB");
        assert_eq!(format_size(1536), "1.5 KB");
        assert_eq!(format_size(1024 * 1024), "1.0 MB");
        assert_eq!(format_size(1024 * 1024 * 1024), "1.0 GB");
    }

    #[test]
    fn test_create_progress_bar() {
        let config = ProgressConfig::default();

        // Should create progress for large operations
        let progress = create_progress_bar(&config, 100, 50 * 1024 * 1024, false);
        assert!(progress.is_some());

        // Should not create progress for small operations
        let progress = create_progress_bar(&config, 5, 1024, false);
        assert!(progress.is_none());

        // Should create progress when forced
        let progress = create_progress_bar(&config, 1, 100, true);
        assert!(progress.is_some());
    }

    #[test]
    fn test_is_ci_environment() {
        // This test may vary depending on environment
        // Just ensure it doesn't panic
        let _is_ci = is_ci_environment();
    }

    // New tests for Progress Display Methods
    #[test]
    fn test_start_scanning() {
        let config = ProgressConfig {
            enabled: true,
            supports_ansi: true,
            is_interactive: true,
            ..ProgressConfig::default()
        };
        let mut progress = BackupProgress::new(config);

        // Initially should be in scanning phase
        assert!(matches!(progress.phase, ProgressPhase::Scanning));
        assert!(progress.progress_bar.is_none());

        // Start scanning should create a progress bar
        progress.start_scanning();
        assert!(progress.progress_bar.is_some());
    }

    #[test]
    fn test_start_scanning_non_interactive() {
        let config = ProgressConfig {
            enabled: true,
            is_interactive: false,
            ..ProgressConfig::default()
        };
        let mut progress = BackupProgress::new(config);

        // Start scanning should not create progress bar for non-interactive
        progress.start_scanning();
        assert!(progress.progress_bar.is_none());
    }

    #[test]
    fn test_update_scan_progress() {
        let config = ProgressConfig {
            enabled: true,
            supports_ansi: true,
            is_interactive: true,
            ..ProgressConfig::default()
        };
        let mut progress = BackupProgress::new(config);
        let test_path = Path::new("/test/file.txt");

        // Test updating scan progress
        progress.update_scan_progress(5, test_path);
        assert_eq!(progress.files_processed, 5);
        assert_eq!(progress.current_file, Some(test_path.to_path_buf()));

        // Test updating with more files
        progress.update_scan_progress(10, test_path);
        assert_eq!(progress.files_processed, 10);
    }

    #[test]
    fn test_finish_scanning() {
        let config = ProgressConfig {
            enabled: true,
            supports_ansi: true,
            is_interactive: true,
            ..ProgressConfig::default()
        };
        let mut progress = BackupProgress::new(config);

        // Initially scanning phase
        assert!(matches!(progress.phase, ProgressPhase::Scanning));

        // Finish scanning should transition to backing phase
        progress.finish_scanning(100, 1024 * 1024);
        assert!(matches!(progress.phase, ProgressPhase::Backing));
        assert_eq!(progress.files_total, Some(100));
        assert_eq!(progress.bytes_total, Some(1024 * 1024));
        assert_eq!(progress.files_processed, 0); // Reset for backing phase
        assert_eq!(progress.bytes_processed, 0); // Reset for backing phase
    }

    #[test]
    fn test_update_backup_progress() {
        let config = ProgressConfig {
            enabled: true,
            supports_ansi: true,
            is_interactive: true,
            ..ProgressConfig::default()
        };
        let mut progress = BackupProgress::new(config);
        let test_path = Path::new("/test/backup_file.txt");

        // Set up backing phase
        progress.finish_scanning(100, 1024 * 1024);

        // Update backup progress
        progress.update_backup_progress(25, 256 * 1024, test_path);
        assert_eq!(progress.files_processed, 25);
        assert_eq!(progress.bytes_processed, 256 * 1024);
        assert_eq!(progress.current_file, Some(test_path.to_path_buf()));

        // Update with more progress
        progress.update_backup_progress(50, 512 * 1024, test_path);
        assert_eq!(progress.files_processed, 50);
        assert_eq!(progress.bytes_processed, 512 * 1024);
    }

    #[test]
    fn test_get_progress_style_different_terminal_widths() {
        // Test that different terminal widths produce valid progress styles
        // We can't examine the template strings directly, but we can test that
        // the method works for different terminal configurations

        let wide_config = ProgressConfig {
            terminal_width: 120,
            ..ProgressConfig::default()
        };
        let wide_progress = BackupProgress::new(wide_config);
        let _wide_style = wide_progress.get_progress_style(); // Should not panic

        let normal_config = ProgressConfig {
            terminal_width: 80,
            ..ProgressConfig::default()
        };
        let normal_progress = BackupProgress::new(normal_config);
        let _normal_style = normal_progress.get_progress_style(); // Should not panic

        let narrow_config = ProgressConfig {
            terminal_width: 60,
            ..ProgressConfig::default()
        };
        let narrow_progress = BackupProgress::new(narrow_config);
        let _narrow_style = narrow_progress.get_progress_style(); // Should not panic

        let very_narrow_config = ProgressConfig {
            terminal_width: 40,
            ..ProgressConfig::default()
        };
        let very_narrow_progress = BackupProgress::new(very_narrow_config);
        let _very_narrow_style = very_narrow_progress.get_progress_style(); // Should not panic
    }

    #[test]
    fn test_format_progress_message_wide_terminal() {
        let config = ProgressConfig {
            terminal_width: 120,
            ..ProgressConfig::default()
        };
        let progress = BackupProgress::new(config);
        let path = Path::new("/some/path/to/myfile.txt");

        let message = progress.format_progress_message(path);
        assert_eq!(message, "Processing: myfile.txt");
    }

    #[test]
    fn test_format_progress_message_narrow_terminal() {
        let config = ProgressConfig {
            terminal_width: 60,
            ..ProgressConfig::default()
        };
        let progress = BackupProgress::new(config);
        let path = Path::new("/some/path/to/very_long_filename_that_should_be_truncated.txt");

        let message = progress.format_progress_message(path);
        // Should truncate long filename
        assert!(message.len() <= 20); // Max length for 60-width terminal
        assert!(message.ends_with("..."));
    }

    #[test]
    fn test_format_progress_message_short_filename() {
        let config = ProgressConfig {
            terminal_width: 60,
            ..ProgressConfig::default()
        };
        let progress = BackupProgress::new(config);
        let path = Path::new("/some/path/to/short.txt");

        let message = progress.format_progress_message(path);
        assert_eq!(message, "short.txt");
    }

    #[test]
    fn test_format_progress_message_no_filename() {
        let config = ProgressConfig {
            terminal_width: 120,
            ..ProgressConfig::default()
        };
        let progress = BackupProgress::new(config);
        let path = Path::new("/");

        let message = progress.format_progress_message(path);
        assert_eq!(message, "Processing: ...");
    }

    #[test]
    fn test_finish_clears_progress_bar() {
        let config = ProgressConfig {
            enabled: true,
            supports_ansi: true,
            is_interactive: true,
            ..ProgressConfig::default()
        };
        let mut progress = BackupProgress::new(config);

        // Start with a progress bar
        progress.start_scanning();
        assert!(progress.progress_bar.is_some());

        // Finish should clear the progress bar
        progress.finish();
        assert!(progress.progress_bar.is_none());
    }
}
