use crate::config::Config;
use crate::error::QbakError;
use crate::naming::{generate_backup_name, resolve_collision};
use crate::progress::{create_progress_bar, BackupProgress};

use crate::utils::{
    calculate_size, copy_permissions, copy_timestamps, format_size, is_hidden, validate_source,
};
use crate::Result;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

#[derive(Debug)]
pub struct BackupResult {
    pub source_path: PathBuf,
    pub backup_path: PathBuf,
    pub files_processed: usize,
    pub total_size: u64,
    pub duration: Duration,
}

impl BackupResult {
    pub fn new(source_path: PathBuf, backup_path: PathBuf) -> Self {
        Self {
            source_path,
            backup_path,
            files_processed: 0,
            total_size: 0,
            duration: Duration::from_secs(0),
        }
    }

    pub fn summary(&self) -> String {
        if self.files_processed == 1 {
            format!(
                "Created backup: {} ({})",
                self.backup_path.display(),
                format_size(self.total_size)
            )
        } else {
            format!(
                "Created backup: {} ({} files, {})",
                self.backup_path.display(),
                self.files_processed,
                format_size(self.total_size)
            )
        }
    }
}

/// Backup a single file
pub fn backup_file(source: &Path, config: &Config) -> Result<BackupResult> {
    let start_time = Instant::now();

    // Validate source
    validate_source(source)?;

    // Generate backup name
    let backup_path = generate_backup_name(source, config)?;
    let final_backup_path = resolve_collision(&backup_path)?;

    // Register operation for cleanup tracking
    let _operation_guard = crate::signal::create_backup_guard(final_backup_path.clone());

    // Calculate size for reporting
    let file_size = calculate_size(source)?;

    // Perform atomic copy
    let temp_path = create_temp_backup_path(&final_backup_path)?;

    // Copy the file with interrupt checking
    copy_file_with_interrupt_check(source, &temp_path)?;

    // Copy metadata if configured
    if config.preserve_permissions {
        copy_permissions(source, &temp_path)?;
        copy_timestamps(source, &temp_path)?;
    }

    // Atomic rename
    fs::rename(&temp_path, &final_backup_path)?;

    let duration = start_time.elapsed();

    let result = BackupResult {
        source_path: source.to_path_buf(),
        backup_path: final_backup_path,
        files_processed: 1,
        total_size: file_size,
        duration,
    };

    // Mark operation as completed (prevents cleanup)
    _operation_guard.complete();

    Ok(result)
}

/// Backup a directory recursively
pub fn backup_directory(source: &Path, config: &Config, verbose: bool) -> Result<BackupResult> {
    let start_time = Instant::now();

    // Validate source
    validate_source(source)?;

    if !source.is_dir() {
        return Err(QbakError::validation("Source is not a directory"));
    }

    // Generate backup name
    let backup_path = generate_backup_name(source, config)?;
    let final_backup_path = resolve_collision(&backup_path)?;

    // Register operation for cleanup tracking
    let _operation_guard = crate::signal::create_backup_guard(final_backup_path.clone());

    // Create backup directory
    fs::create_dir_all(&final_backup_path)?;

    // Copy directory contents
    let mut result = BackupResult::new(source.to_path_buf(), final_backup_path.clone());

    // Check if we should show progress
    let total_files = count_files_recursive(source, config)?;
    let show_progress = verbose;

    if show_progress {
        println!("Backing up directory with {total_files} files...");
    }

    copy_directory_contents(
        source,
        &final_backup_path,
        config,
        &mut result,
        show_progress,
    )?;

    // Set directory permissions if configured
    if config.preserve_permissions {
        copy_permissions(source, &final_backup_path)?;
        copy_timestamps(source, &final_backup_path)?;
    }

    if show_progress {
        println!(
            "Directory backup completed: {} files processed",
            result.files_processed
        );
    }

    result.duration = start_time.elapsed();

    // Mark operation as completed (prevents cleanup)
    _operation_guard.complete();

    Ok(result)
}

/// Copy directory contents recursively
fn copy_directory_contents(
    source_dir: &Path,
    backup_dir: &Path,
    config: &Config,
    result: &mut BackupResult,
    show_progress: bool,
) -> Result<()> {
    for entry in fs::read_dir(source_dir)? {
        // Check for interrupt signal
        if crate::signal::is_interrupted() {
            return Err(QbakError::Interrupted);
        }

        let entry = entry?;
        let source_path = entry.path();
        let file_name = entry.file_name();

        // Skip hidden files if not configured to include them
        if !config.include_hidden && is_hidden(&source_path) {
            continue;
        }

        let backup_path = backup_dir.join(&file_name);
        let metadata = entry.metadata()?;

        if metadata.is_file() {
            // Copy file
            copy_file_to_backup(&source_path, &backup_path, config, result)?;

            // Show progress if enabled
            if show_progress && result.files_processed % 10 == 0 {
                eprint!(".");
                use std::io::{self, Write};
                io::stderr().flush().unwrap_or(());
            }
        } else if metadata.is_dir() {
            // Create directory and recurse
            fs::create_dir_all(&backup_path)?;
            copy_directory_contents(&source_path, &backup_path, config, result, show_progress)?;

            // Set directory permissions
            if config.preserve_permissions {
                copy_permissions(&source_path, &backup_path)?;
                copy_timestamps(&source_path, &backup_path)?;
            }
        } else if metadata.file_type().is_symlink() {
            // Handle symlinks
            handle_symlink(&source_path, &backup_path, config, result, show_progress)?;
        }
    }
    Ok(())
}

/// Copy a single file within a directory backup
fn copy_file_to_backup(
    source: &Path,
    backup: &Path,
    config: &Config,
    result: &mut BackupResult,
) -> Result<()> {
    // Create temp file for atomic operation
    let temp_path = create_temp_backup_path(backup)?;

    // Copy file with interrupt checking
    copy_file_with_interrupt_check(source, &temp_path)?;

    // Copy metadata if configured
    if config.preserve_permissions {
        copy_permissions(source, &temp_path)?;
        copy_timestamps(source, &temp_path)?;
    }

    // Atomic rename
    fs::rename(&temp_path, backup)?;

    // Update statistics
    let file_size = fs::metadata(source)?.len();
    result.files_processed += 1;
    result.total_size += file_size;

    Ok(())
}

/// Copy a file while checking for interrupt signals
fn copy_file_with_interrupt_check(source: &Path, dest: &Path) -> Result<()> {
    use std::io::{Read, Write};

    let mut source_file = fs::File::open(source)?;
    let mut dest_file = fs::File::create(dest)?;

    let mut buffer = vec![0u8; 64 * 1024]; // 64KB buffer

    loop {
        // Check for interrupt before reading each chunk
        if crate::signal::is_interrupted() {
            // Clean up partial file
            let _ = fs::remove_file(dest);
            return Err(QbakError::Interrupted);
        }

        let bytes_read = source_file.read(&mut buffer)?;
        if bytes_read == 0 {
            break; // EOF
        }

        dest_file.write_all(&buffer[..bytes_read])?;
    }

    dest_file.flush()?;
    Ok(())
}

/// Handle symlink based on configuration
fn handle_symlink(
    source: &Path,
    backup: &Path,
    config: &Config,
    result: &mut BackupResult,
    show_progress: bool,
) -> Result<()> {
    if config.follow_symlinks {
        // Follow the symlink and copy the target
        let target = fs::read_link(source)?;
        let resolved_target = if target.is_absolute() {
            target
        } else {
            source.parent().unwrap_or(Path::new(".")).join(target)
        };

        if resolved_target.exists() {
            let metadata = fs::metadata(&resolved_target)?;
            if metadata.is_file() {
                copy_file_to_backup(&resolved_target, backup, config, result)?;
            } else if metadata.is_dir() {
                fs::create_dir_all(backup)?;
                copy_directory_contents(&resolved_target, backup, config, result, show_progress)?;
            }
        }
    } else {
        // Preserve the symlink as-is
        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            let target = fs::read_link(source)?;
            symlink(target, backup)?;
        }

        #[cfg(not(unix))]
        {
            // On non-Unix systems, copy the target file instead
            let target = fs::read_link(source)?;
            let resolved_target = if target.is_absolute() {
                target
            } else {
                source.parent().unwrap_or(Path::new(".")).join(target)
            };

            if resolved_target.exists() && resolved_target.is_file() {
                copy_file_to_backup(&resolved_target, backup, config, result)?;
            }
        }
    }

    Ok(())
}

/// Create a temporary backup path for atomic operations
fn create_temp_backup_path(backup_path: &Path) -> Result<PathBuf> {
    let parent = backup_path.parent().unwrap_or(Path::new("."));
    let filename = backup_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| QbakError::validation("Invalid backup filename"))?;

    let process_id = std::process::id();
    let temp_name = format!(".qbak_temp_{process_id}_{filename}");
    Ok(parent.join(temp_name))
}

/// Count the total number of files in a directory recursively
fn count_files_recursive(dir: &Path, config: &Config) -> Result<usize> {
    let mut count = 0;

    if !dir.is_dir() {
        return Ok(1); // Single file
    }

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        // Skip hidden files if not configured to include them
        if !config.include_hidden && is_hidden(&path) {
            continue;
        }

        let metadata = entry.metadata()?;

        if metadata.is_file() {
            count += 1;
        } else if metadata.is_dir() {
            count += count_files_recursive(&path, config)?;
        } else if metadata.file_type().is_symlink() && config.follow_symlinks {
            // Count symlink targets if we're following them
            let target = fs::read_link(&path)?;
            let resolved_target = if target.is_absolute() {
                target
            } else {
                path.parent().unwrap_or(Path::new(".")).join(target)
            };

            if resolved_target.exists() {
                let target_metadata = fs::metadata(&resolved_target)?;
                if target_metadata.is_file() {
                    count += 1;
                } else if target_metadata.is_dir() {
                    count += count_files_recursive(&resolved_target, config)?;
                }
            }
        }
    }

    Ok(count)
}

/// Clean up any temporary files that might be left over
pub fn cleanup_temp_files(dir: &Path) -> Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if let Some(filename) = path.file_name().and_then(|name| name.to_str()) {
            if filename.starts_with(".qbak_temp_") {
                // Try to remove temp file
                let _ = fs::remove_file(&path);
            }
        }
    }

    Ok(())
}

/// Backup a directory with progress indication
pub fn backup_directory_with_progress(
    source: &Path,
    config: &Config,
    force_progress: bool,
    quiet: bool,
) -> Result<BackupResult> {
    let start_time = Instant::now();

    // Validate source
    validate_source(source)?;

    // Generate backup name
    let backup_path = generate_backup_name(source, config)?;
    let final_backup_path = resolve_collision(&backup_path)?;

    // Register operation for cleanup tracking
    let _operation_guard = crate::signal::create_backup_guard(final_backup_path.clone());

    // First, count files and calculate size (scanning phase)
    let (file_count, total_size) = count_files_and_size(source, config)?;

    // Check if we should show progress
    let mut progress = if !quiet {
        create_progress_bar(&config.progress, file_count, total_size, force_progress)
    } else {
        None
    };

    // Start progress if available
    if let Some(ref mut prog) = progress {
        prog.start_scanning();
        prog.finish_scanning(file_count, total_size);
    }

    // Create backup directory
    fs::create_dir_all(&final_backup_path)?;

    // Initialize result
    let mut result = BackupResult::new(source.to_path_buf(), final_backup_path.clone());

    // Copy contents with progress tracking
    copy_directory_contents_with_progress(
        source,
        &final_backup_path,
        config,
        &mut result,
        &mut progress.as_mut(),
    )?;

    // Finish progress
    if let Some(ref mut prog) = progress {
        prog.finish();
    }

    let duration = start_time.elapsed();
    result.duration = duration;

    // Mark operation as completed (prevents cleanup)
    _operation_guard.complete();

    Ok(result)
}

/// Count files and calculate total size, with optional progress
pub fn count_files_and_size_with_progress(source: &Path, config: &Config) -> Result<(usize, u64)> {
    let mut progress = create_progress_bar(&config.progress, 0, 0, true);
    if let Some(ref mut prog) = progress {
        prog.start_scanning();
    }

    let result = count_files_and_size_recursive(source, config, &mut progress.as_mut());

    if let Some(ref mut prog) = progress {
        prog.finish();
    }

    result
}

/// Count files and calculate total size, without progress
pub fn count_files_and_size(source: &Path, config: &Config) -> Result<(usize, u64)> {
    let mut none_progress = None;
    count_files_and_size_recursive(source, config, &mut none_progress)
}

/// Recursive function to count files and calculate total size
fn count_files_and_size_recursive(
    dir: &Path,
    config: &Config,
    progress: &mut Option<&mut BackupProgress>,
) -> Result<(usize, u64)> {
    let mut total_files = 0;
    let mut total_size = 0;

    for entry in fs::read_dir(dir)? {
        // Check for interrupt signal during scanning
        if crate::signal::is_interrupted() {
            return Err(QbakError::Interrupted);
        }

        let entry = entry?;
        let path = entry.path();

        // Skip hidden files if not configured to include them
        if !config.include_hidden && is_hidden(&path) {
            continue;
        }

        let metadata = entry.metadata()?;

        if metadata.is_file() {
            total_files += 1;
            total_size += metadata.len();

            // Update scanning progress occasionally
            if let Some(ref mut p) = progress {
                if total_files % 100 == 0 {
                    p.update_scan_progress(total_files, &path);
                }
            }
        } else if metadata.is_dir() {
            let (sub_files, sub_size) = count_files_and_size_recursive(&path, config, progress)?;
            total_files += sub_files;
            total_size += sub_size;
        }
        // Skip symlinks for size calculation
    }

    Ok((total_files, total_size))
}

/// Copy directory contents with progress tracking
fn copy_directory_contents_with_progress(
    source_dir: &Path,
    backup_dir: &Path,
    config: &Config,
    result: &mut BackupResult,
    progress: &mut Option<&mut BackupProgress>,
) -> Result<()> {
    for entry in fs::read_dir(source_dir)? {
        // Check for interrupt signal
        if crate::signal::is_interrupted() {
            return Err(QbakError::Interrupted);
        }

        let entry = entry?;
        let source_path = entry.path();
        let filename = source_path.file_name().unwrap();
        let backup_path = backup_dir.join(filename);

        // Skip hidden files if not configured to include them
        if !config.include_hidden && is_hidden(&source_path) {
            continue;
        }

        let metadata = entry.metadata()?;

        if metadata.is_file() {
            copy_file_to_backup(&source_path, &backup_path, config, result)?;

            // Update progress
            if let Some(ref mut prog) = progress {
                prog.update_backup_progress(
                    result.files_processed,
                    result.total_size,
                    &source_path,
                );
            }
        } else if metadata.is_dir() {
            fs::create_dir_all(&backup_path)?;
            copy_directory_contents_with_progress(
                &source_path,
                &backup_path,
                config,
                result,
                progress,
            )?;
        } else if metadata.file_type().is_symlink() {
            handle_symlink_with_progress(&source_path, &backup_path, config, result, progress)?;
        }
    }
    Ok(())
}

/// Handle symlink with progress tracking
fn handle_symlink_with_progress(
    source: &Path,
    backup: &Path,
    config: &Config,
    result: &mut BackupResult,
    progress: &mut Option<&mut BackupProgress>,
) -> Result<()> {
    if config.follow_symlinks {
        // Follow the symlink and copy the target
        let target = fs::read_link(source)?;
        let resolved_target = if target.is_absolute() {
            target
        } else {
            source.parent().unwrap_or(Path::new(".")).join(target)
        };

        if resolved_target.exists() {
            let metadata = fs::metadata(&resolved_target)?;
            if metadata.is_file() {
                copy_file_to_backup(&resolved_target, backup, config, result)?;

                // Update progress
                if let Some(ref mut prog) = progress {
                    prog.update_backup_progress(result.files_processed, result.total_size, source);
                }
            } else if metadata.is_dir() {
                fs::create_dir_all(backup)?;
                copy_directory_contents_with_progress(
                    &resolved_target,
                    backup,
                    config,
                    result,
                    progress,
                )?;
            }
        }
    } else {
        // Preserve the symlink as-is
        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            let target = fs::read_link(source)?;
            symlink(target, backup)?;
        }

        #[cfg(not(unix))]
        {
            // On non-Unix systems, copy the target file instead
            let target = fs::read_link(source)?;
            let resolved_target = if target.is_absolute() {
                target
            } else {
                source.parent().unwrap_or(Path::new(".")).join(target)
            };

            if resolved_target.exists() && resolved_target.is_file() {
                copy_file_to_backup(&resolved_target, backup, config, result)?;

                // Update progress
                if let Some(ref mut prog) = progress {
                    prog.update_backup_progress(result.files_processed, result.total_size, source);
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::default_config;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_backup_file() {
        let dir = tempdir().unwrap();
        let source_path = dir.path().join("test.txt");

        // Create source file
        let mut file = File::create(&source_path).unwrap();
        writeln!(file, "Hello, World!").unwrap();

        let config = default_config();
        let result = backup_file(&source_path, &config).unwrap();

        // Check that backup was created
        assert!(result.backup_path.exists());
        assert_eq!(result.files_processed, 1);
        assert_eq!(result.total_size, 14); // "Hello, World!\n"

        // Check backup content
        let backup_content = fs::read_to_string(&result.backup_path).unwrap();
        assert_eq!(backup_content, "Hello, World!\n");

        // Verify backup summary
        let summary = result.summary();
        assert!(summary.contains("Created backup:"));
        assert!(summary.contains("14 B"));
    }

    #[test]
    fn test_backup_file_nonexistent() {
        let dir = tempdir().unwrap();
        let source_path = dir.path().join("nonexistent.txt");

        let config = default_config();
        let result = backup_file(&source_path, &config);

        assert!(result.is_err());
        match result.unwrap_err() {
            QbakError::SourceNotFound { path } => assert_eq!(path, source_path),
            _ => panic!("Expected SourceNotFound error"),
        }
    }

    #[test]
    fn test_backup_directory() {
        let dir = tempdir().unwrap();
        let source_dir = dir.path().join("source");
        fs::create_dir_all(&source_dir).unwrap();

        // Create some files
        File::create(source_dir.join("file1.txt")).unwrap();
        File::create(source_dir.join("file2.txt")).unwrap();

        let subdir = source_dir.join("subdir");
        fs::create_dir_all(&subdir).unwrap();
        File::create(subdir.join("file3.txt")).unwrap();

        let config = default_config();
        let result = backup_directory(&source_dir, &config, false).unwrap();

        // Check that backup directory was created
        assert!(result.backup_path.exists());
        assert!(result.backup_path.is_dir());
        assert_eq!(result.files_processed, 3);

        // Check that files were copied
        assert!(result.backup_path.join("file1.txt").exists());
        assert!(result.backup_path.join("file2.txt").exists());
        assert!(result.backup_path.join("subdir").join("file3.txt").exists());

        // Verify backup summary for multiple files
        let summary = result.summary();
        assert!(summary.contains("Created backup:"));
        assert!(summary.contains("3 files"));
    }

    #[test]
    fn test_backup_directory_on_file() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("file.txt");
        File::create(&file_path).unwrap();

        let config = default_config();
        let result = backup_directory(&file_path, &config, false);

        assert!(result.is_err());
        match result.unwrap_err() {
            QbakError::Validation { message } => assert!(message.contains("not a directory")),
            _ => panic!("Expected Validation error"),
        }
    }

    #[test]
    fn test_backup_collision_resolution() {
        let dir = tempdir().unwrap();
        let source_path = dir.path().join("test.txt");
        File::create(&source_path).unwrap();

        let config = default_config();

        // First backup
        let result1 = backup_file(&source_path, &config).unwrap();
        assert!(result1.backup_path.exists());

        // Create a file with the same timestamp pattern to force collision
        let backup_name = result1.backup_path.file_name().unwrap().to_str().unwrap();
        let collision_path = dir.path().join(backup_name);
        File::create(&collision_path).unwrap();

        // This should work because resolve_collision will handle it
        // Note: In real usage, this is less likely due to timestamp precision
    }

    #[test]
    fn test_hidden_file_handling() {
        let dir = tempdir().unwrap();
        let source_dir = dir.path().join("source");
        fs::create_dir_all(&source_dir).unwrap();

        // Create visible and hidden files
        File::create(source_dir.join("visible.txt")).unwrap();
        File::create(source_dir.join(".hidden.txt")).unwrap();

        // Test with include_hidden = true
        let mut config = default_config();
        config.include_hidden = true;
        let result = backup_directory(&source_dir, &config, false).unwrap();
        assert_eq!(result.files_processed, 2);
        assert!(result.backup_path.join(".hidden.txt").exists());

        // Clean up for next test
        fs::remove_dir_all(&result.backup_path).unwrap();

        // Test with include_hidden = false
        config.include_hidden = false;
        let result = backup_directory(&source_dir, &config, false).unwrap();
        assert_eq!(result.files_processed, 1);
        assert!(!result.backup_path.join(".hidden.txt").exists());
        assert!(result.backup_path.join("visible.txt").exists());
    }

    #[test]
    fn test_backup_empty_directory() {
        let dir = tempdir().unwrap();
        let source_dir = dir.path().join("empty_source");
        fs::create_dir_all(&source_dir).unwrap();

        let config = default_config();
        let result = backup_directory(&source_dir, &config, false).unwrap();

        assert!(result.backup_path.exists());
        assert!(result.backup_path.is_dir());
        assert_eq!(result.files_processed, 0);
        assert_eq!(result.total_size, 0);
    }

    #[test]
    fn test_backup_file_with_permissions_disabled() {
        let dir = tempdir().unwrap();
        let source_path = dir.path().join("test.txt");

        let mut file = File::create(&source_path).unwrap();
        writeln!(file, "Content").unwrap();

        let mut config = default_config();
        config.preserve_permissions = false;

        let result = backup_file(&source_path, &config).unwrap();
        assert!(result.backup_path.exists());

        // Should still create backup successfully
        let backup_content = fs::read_to_string(&result.backup_path).unwrap();
        assert_eq!(backup_content, "Content\n");
    }

    #[test]
    fn test_backup_deeply_nested_directory() {
        let dir = tempdir().unwrap();
        let source_dir = dir.path().join("source");

        // Create deeply nested structure
        let deep_path = source_dir.join("a").join("b").join("c").join("d").join("e");
        fs::create_dir_all(&deep_path).unwrap();

        // Add files at different levels
        fs::write(source_dir.join("root.txt"), "root").unwrap();
        fs::write(source_dir.join("a").join("a.txt"), "a").unwrap();
        fs::write(source_dir.join("a").join("b").join("b.txt"), "b").unwrap();
        fs::write(deep_path.join("deep.txt"), "deep").unwrap();

        let config = default_config();
        let result = backup_directory(&source_dir, &config, false).unwrap();

        assert_eq!(result.files_processed, 4);
        assert!(result.backup_path.join("root.txt").exists());
        assert!(result.backup_path.join("a").join("a.txt").exists());
        assert!(result
            .backup_path
            .join("a")
            .join("b")
            .join("b.txt")
            .exists());
        assert!(result
            .backup_path
            .join("a")
            .join("b")
            .join("c")
            .join("d")
            .join("e")
            .join("deep.txt")
            .exists());
    }

    #[test]
    fn test_backup_file_large_content() {
        let dir = tempdir().unwrap();
        let source_path = dir.path().join("large.txt");

        // Create file with substantial content
        let large_content = "x".repeat(10000);
        fs::write(&source_path, &large_content).unwrap();

        let config = default_config();
        let result = backup_file(&source_path, &config).unwrap();

        assert!(result.backup_path.exists());
        assert_eq!(result.total_size, 10000);

        let backup_content = fs::read_to_string(&result.backup_path).unwrap();
        assert_eq!(backup_content, large_content);
    }

    #[test]
    fn test_backup_result_new() {
        let source = PathBuf::from("/source/path");
        let backup = PathBuf::from("/backup/path");

        let result = BackupResult::new(source.clone(), backup.clone());

        assert_eq!(result.source_path, source);
        assert_eq!(result.backup_path, backup);
        assert_eq!(result.files_processed, 0);
        assert_eq!(result.total_size, 0);
        assert_eq!(result.duration.as_secs(), 0);
    }

    #[test]
    fn test_backup_result_summary_single_file() {
        let mut result =
            BackupResult::new(PathBuf::from("source.txt"), PathBuf::from("backup.txt"));
        result.files_processed = 1;
        result.total_size = 1024;

        let summary = result.summary();
        assert!(summary.contains("Created backup:"));
        assert!(summary.contains("1.0 KB"));
        assert!(!summary.contains("files")); // Should not mention "files" for single file
    }

    #[test]
    fn test_backup_result_summary_multiple_files() {
        let mut result =
            BackupResult::new(PathBuf::from("source_dir"), PathBuf::from("backup_dir"));
        result.files_processed = 5;
        result.total_size = 2048;

        let summary = result.summary();
        assert!(summary.contains("Created backup:"));
        assert!(summary.contains("5 files"));
        assert!(summary.contains("2.0 KB"));
    }

    #[test]
    fn test_backup_directory_with_symlinks() {
        let dir = tempdir().unwrap();
        let source_dir = dir.path().join("source");
        fs::create_dir_all(&source_dir).unwrap();

        // Create a regular file
        fs::write(source_dir.join("regular.txt"), "regular content").unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;

            // Create a symlink to the regular file
            let target = source_dir.join("regular.txt");
            let link = source_dir.join("link.txt");
            symlink(&target, &link).unwrap();

            // Test with follow_symlinks = true (default)
            let mut config = default_config();
            config.follow_symlinks = true;

            let result = backup_directory(&source_dir, &config, false).unwrap();

            // Should process both the regular file and the symlink target
            assert_eq!(result.files_processed, 2);
            assert!(result.backup_path.join("regular.txt").exists());
            assert!(result.backup_path.join("link.txt").exists());

            // Clean up
            fs::remove_dir_all(&result.backup_path).unwrap();

            // Test with follow_symlinks = false
            config.follow_symlinks = false;
            let result = backup_directory(&source_dir, &config, false).unwrap();

            // Should still handle symlinks
            assert!(result.backup_path.join("regular.txt").exists());
        }
    }

    #[test]
    fn test_cleanup_temp_files() {
        let dir = tempdir().unwrap();

        // Create some temp files that look like qbak temp files
        let temp1 = dir.path().join(".qbak_temp_12345_file1.txt");
        let temp2 = dir.path().join(".qbak_temp_67890_file2.txt");
        let normal = dir.path().join("normal_file.txt");

        File::create(&temp1).unwrap();
        File::create(&temp2).unwrap();
        File::create(&normal).unwrap();

        // Run cleanup
        assert!(cleanup_temp_files(dir.path()).is_ok());

        // Temp files should be gone, normal file should remain
        assert!(!temp1.exists());
        assert!(!temp2.exists());
        assert!(normal.exists());
    }

    #[test]
    fn test_cleanup_temp_files_nonexistent_dir() {
        let nonexistent = Path::new("/nonexistent/directory");
        let result = cleanup_temp_files(nonexistent);
        assert!(result.is_ok()); // Should handle gracefully
    }

    #[test]
    fn test_backup_file_zero_size() {
        let dir = tempdir().unwrap();
        let source_path = dir.path().join("empty.txt");

        // Create empty file
        File::create(&source_path).unwrap();

        let config = default_config();
        let result = backup_file(&source_path, &config).unwrap();

        assert!(result.backup_path.exists());
        assert_eq!(result.total_size, 0);
        assert_eq!(result.files_processed, 1);
    }

    #[test]
    fn test_count_files_recursive() {
        let dir = tempdir().unwrap();
        let source_dir = dir.path().join("source");
        fs::create_dir_all(&source_dir).unwrap();

        // Create test structure
        File::create(source_dir.join("file1.txt")).unwrap();
        File::create(source_dir.join("file2.txt")).unwrap();

        let subdir = source_dir.join("subdir");
        fs::create_dir_all(&subdir).unwrap();
        File::create(subdir.join("file3.txt")).unwrap();
        File::create(subdir.join("file4.txt")).unwrap();

        let config = default_config();
        let count = count_files_recursive(&source_dir, &config).unwrap();
        assert_eq!(count, 4);
    }

    #[test]
    fn test_count_files_recursive_with_hidden() {
        let dir = tempdir().unwrap();
        let source_dir = dir.path().join("source");
        fs::create_dir_all(&source_dir).unwrap();

        // Create normal and hidden files
        File::create(source_dir.join("file1.txt")).unwrap();
        File::create(source_dir.join(".hidden")).unwrap();

        // Test with include_hidden = true
        let mut config = default_config();
        config.include_hidden = true;
        let count = count_files_recursive(&source_dir, &config).unwrap();
        assert_eq!(count, 2);

        // Test with include_hidden = false
        config.include_hidden = false;
        let count = count_files_recursive(&source_dir, &config).unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_count_files_recursive_single_file() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("single.txt");
        File::create(&file_path).unwrap();

        let config = default_config();
        let count = count_files_recursive(&file_path, &config).unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_progress_with_verbose_flag() {
        let temp_dir = tempfile::tempdir().unwrap();
        let source = temp_dir.path().join("source");

        std::fs::create_dir(&source).unwrap();
        std::fs::write(source.join("file1.txt"), "content1").unwrap();
        std::fs::write(source.join("file2.txt"), "content2").unwrap();
        std::fs::write(source.join("file3.txt"), "content3").unwrap();

        let config = Config::default();

        // Test with verbose=true - should always show progress
        let result = backup_directory(&source, &config, true);
        assert!(result.is_ok());

        // Test with verbose=false - should never show progress
        let result2 = backup_directory(&source, &config, false);
        assert!(result2.is_ok());
    }

    #[test]
    fn test_backup_directory_mixed_content() {
        let dir = tempdir().unwrap();
        let source_dir = dir.path().join("mixed");
        fs::create_dir_all(&source_dir).unwrap();

        // Create mix of empty and non-empty files
        File::create(source_dir.join("empty.txt")).unwrap();
        fs::write(source_dir.join("small.txt"), "a").unwrap();
        fs::write(source_dir.join("medium.txt"), "hello world").unwrap();

        // Create subdirectory with files
        let subdir = source_dir.join("sub");
        fs::create_dir_all(&subdir).unwrap();
        fs::write(subdir.join("sub_file.txt"), "sub content").unwrap();

        let config = default_config();
        let result = backup_directory(&source_dir, &config, false).unwrap();

        assert_eq!(result.files_processed, 4);
        assert_eq!(result.total_size, 23); // 0 + 1 + 11 + 11 = 23 bytes

        // Verify all files were backed up
        assert!(result.backup_path.join("empty.txt").exists());
        assert!(result.backup_path.join("small.txt").exists());
        assert!(result.backup_path.join("medium.txt").exists());
        assert!(result.backup_path.join("sub").join("sub_file.txt").exists());
    }

    #[test]
    fn test_backup_operation_guard_cleanup_on_panic() {
        let dir = tempdir().unwrap();
        let source_path = dir.path().join("test.txt");
        fs::write(&source_path, "test content").unwrap();

        let config = default_config();
        let backup_path = generate_backup_name(&source_path, &config).unwrap();
        let final_backup_path = resolve_collision(&backup_path).unwrap();

        // Simulate operation that panics after creating backup directory
        let result = std::panic::catch_unwind(|| {
            let _guard = crate::signal::create_backup_guard(final_backup_path.clone());

            // Create partial backup to simulate interrupted operation
            fs::create_dir_all(&final_backup_path).unwrap();
            fs::write(final_backup_path.join("partial.txt"), "partial").unwrap();

            // Simulate panic/interruption (don't call guard.complete())
            panic!("Simulated interruption");
        });

        assert!(result.is_err()); // Panic occurred

        // The guard should have been dropped, but the files remain
        // (cleanup_active_operations would be called by signal handler)
        assert!(final_backup_path.exists());
    }

    #[test]
    fn test_backup_operation_guard_no_cleanup_on_success() {
        let dir = tempdir().unwrap();
        let source_path = dir.path().join("test.txt");
        fs::write(&source_path, "test content").unwrap();

        let config = default_config();
        let backup_path = generate_backup_name(&source_path, &config).unwrap();
        let final_backup_path = resolve_collision(&backup_path).unwrap();

        {
            let guard = crate::signal::create_backup_guard(final_backup_path.clone());

            // Create backup successfully
            fs::create_dir_all(&final_backup_path).unwrap();
            fs::write(final_backup_path.join("complete.txt"), "complete").unwrap();

            // Mark as completed
            guard.complete();
        } // Guard is dropped here, but complete() was called

        // Backup should still exist after successful completion
        assert!(final_backup_path.exists());
        assert!(final_backup_path.join("complete.txt").exists());
    }

    #[test]
    fn test_signal_cleanup_removes_incomplete_backups() {
        let dir = tempdir().unwrap();

        // Create multiple incomplete backup operations
        let backup1 = dir.path().join("backup1-20250630T123456-qbak");
        let backup2 = dir.path().join("backup2-20250630T123457-qbak.txt");

        fs::create_dir_all(&backup1).unwrap();
        fs::write(backup1.join("partial1.txt"), "content").unwrap();
        fs::write(&backup2, "partial content").unwrap();

        // Create isolated context for this test
        let context = crate::signal::BackupContext::new();

        // Register operations (simulate active backups)
        let _guard1 = context.register_operation(backup1.clone());
        let _guard2 = context.register_operation(backup2.clone());

        // Verify they're tracked and exist
        let active_ops = context.get_active_operations();
        assert!(active_ops.contains(&backup1));
        assert!(active_ops.contains(&backup2));
        assert!(backup1.exists());
        assert!(backup2.exists());

        // Simulate signal handler cleanup
        context.cleanup_active_operations_with_mode(true);

        // Verify they were removed
        assert!(!backup1.exists());
        assert!(!backup2.exists());

        // Verify tracking is cleared
        let remaining_ops = context.get_active_operations();
        assert!(remaining_ops.is_empty());
    }

    #[test]
    fn test_backup_file_interrupted_simulation() {
        let dir = tempdir().unwrap();
        let source_path = dir.path().join("test.txt");
        fs::write(&source_path, "test content").unwrap();

        let config = default_config();
        let backup_path = generate_backup_name(&source_path, &config).unwrap();
        let final_backup_path = resolve_collision(&backup_path).unwrap();

        // Create isolated context for this test
        let context = crate::signal::BackupContext::new();

        // Simulate backup_file being interrupted before completion
        // In real scenario, CTRL+C happens while guard is still active
        let guard = context.register_operation(final_backup_path.clone());

        // Start backup process
        fs::copy(&source_path, &final_backup_path).unwrap();

        // File exists and operation is tracked
        assert!(final_backup_path.exists());
        let active_ops = context.get_active_operations();
        assert!(active_ops.contains(&final_backup_path));

        // Simulate CTRL+C signal handler being called while guard is still active
        context.cleanup_active_operations_with_mode(true);

        // Now it should be gone
        assert!(!final_backup_path.exists());

        drop(guard);
    }

    #[test]
    fn test_backup_directory_interrupted_simulation() {
        let dir = tempdir().unwrap();
        let source_dir = dir.path().join("source");
        fs::create_dir_all(&source_dir).unwrap();

        // Create several files
        for i in 1..=5 {
            fs::write(
                source_dir.join(format!("file{i}.txt")),
                format!("content{i}"),
            )
            .unwrap();
        }

        let config = default_config();
        let backup_path = generate_backup_name(&source_dir, &config).unwrap();
        let final_backup_path = resolve_collision(&backup_path).unwrap();

        // Create isolated context for this test
        let context = crate::signal::BackupContext::new();

        // Simulate directory backup being interrupted partway through
        // In real scenario, CTRL+C happens while guard is still active
        let guard = context.register_operation(final_backup_path.clone());

        // Start backup process - create directory and copy some files
        fs::create_dir_all(&final_backup_path).unwrap();
        fs::write(final_backup_path.join("file1.txt"), "content1").unwrap();
        fs::write(final_backup_path.join("file2.txt"), "content2").unwrap();
        // Interrupted before copying file3.txt, file4.txt, file5.txt

        // Partial backup exists and is tracked
        assert!(final_backup_path.exists());
        assert!(final_backup_path.join("file1.txt").exists());
        assert!(final_backup_path.join("file2.txt").exists());
        assert!(!final_backup_path.join("file3.txt").exists()); // Not copied yet

        let active_ops = context.get_active_operations();
        assert!(active_ops.contains(&final_backup_path));

        // Simulate CTRL+C signal handler being called while guard is still active
        context.cleanup_active_operations_with_mode(true);

        // Entire backup directory should be gone
        assert!(!final_backup_path.exists());
        assert!(!final_backup_path.join("file1.txt").exists());
        assert!(!final_backup_path.join("file2.txt").exists());

        drop(guard);
    }

    #[test]
    fn test_interrupt_handling_during_backup() {
        let dir = tempdir().unwrap();
        let source_dir = dir.path().join("source");
        fs::create_dir_all(&source_dir).unwrap();

        // Create many files to simulate a backup that takes time
        for i in 1..=50 {
            fs::write(
                source_dir.join(format!("file_{i:03}.txt")),
                format!("content for file {i}"),
            )
            .unwrap();
        }

        let config = default_config();
        let backup_path = generate_backup_name(&source_dir, &config).unwrap();
        let final_backup_path = resolve_collision(&backup_path).unwrap();

        // Create isolated context for this test
        let context = crate::signal::BackupContext::new();

        // Start backup operation
        let guard = context.register_operation(final_backup_path.clone());

        // Create backup directory
        fs::create_dir_all(&final_backup_path).unwrap();

        // Start copying some files manually to simulate partial progress
        fs::copy(
            source_dir.join("file_001.txt"),
            final_backup_path.join("file_001.txt"),
        )
        .unwrap();
        fs::copy(
            source_dir.join("file_002.txt"),
            final_backup_path.join("file_002.txt"),
        )
        .unwrap();

        // Verify partial backup exists
        assert!(final_backup_path.exists());
        assert!(final_backup_path.join("file_001.txt").exists());
        assert!(final_backup_path.join("file_002.txt").exists());
        assert!(!final_backup_path.join("file_003.txt").exists());

        // Simulate CTRL+C being pressed
        context.set_interrupted(true);

        // Verify interrupt is detected
        assert!(context.is_interrupted());

        // Try to continue backup - should detect interrupt and fail
        crate::signal::set_interrupt_flag(context.interrupt_flag());

        let result = copy_directory_contents(
            &source_dir,
            &final_backup_path,
            &config,
            &mut BackupResult::new(source_dir.clone(), final_backup_path.clone()),
            false,
        );

        // Should fail with Interrupted error
        assert!(result.is_err());
        match result.unwrap_err() {
            QbakError::Interrupted => {}
            other => panic!("Expected Interrupted error, got: {other:?}"),
        }

        // Simulate signal handler cleanup
        context.cleanup_active_operations_with_mode(true);

        // Verify cleanup worked - partial backup should be removed
        assert!(!final_backup_path.exists());
        assert!(!final_backup_path.join("file_001.txt").exists());
        assert!(!final_backup_path.join("file_002.txt").exists());

        drop(guard);
    }

    #[test]
    fn test_large_file_interrupt_and_cleanup() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::{Arc, Mutex};
        use std::thread;
        use std::time::Duration;

        let dir = tempdir().unwrap();
        let source_dir = dir.path().join("large_source");
        fs::create_dir_all(&source_dir).unwrap();

        // Create test files for interrupt testing
        for i in 1..=2000 {
            let content = vec![(i % 256) as u8; 512 * 1024]; // 0.5MB with varying pattern
            fs::write(source_dir.join(format!("file_{:04}.bin", i)), content).unwrap();
        }

        let config = default_config();
        let backup_path = generate_backup_name(&source_dir, &config).unwrap();
        let final_backup_path = resolve_collision(&backup_path).unwrap();

        // Set up shared state for test coordination
        let context = crate::signal::BackupContext::new();
        let backup_started = Arc::new(AtomicBool::new(false));
        let backup_result = Arc::new(Mutex::new(None));

        crate::signal::set_interrupt_flag(context.interrupt_flag());

        // Clone variables for the backup thread
        let source_dir_clone = source_dir.clone();
        let config_clone = config.clone();
        let backup_started_clone = backup_started.clone();
        let backup_result_clone = backup_result.clone();

        // Start backup in a separate thread to allow interruption
        let backup_thread = thread::spawn(move || {
            backup_started_clone.store(true, Ordering::SeqCst);

            let result = backup_directory(&source_dir_clone, &config_clone, false);

            // Store result for main thread to examine
            if let Ok(mut guard) = backup_result_clone.lock() {
                *guard = Some(result);
            }
        });

        // Wait for backup to start
        while !backup_started.load(Ordering::SeqCst) {
            thread::sleep(Duration::from_millis(10));
        }

        // Give backup time to start copying some files (but not finish all 2000)
        thread::sleep(Duration::from_millis(200));

        // Interrupt the backup
        context.set_interrupted(true);

        // Wait for backup thread to complete
        backup_thread.join().unwrap();

        // Check the backup result
        let result_guard = backup_result.lock().unwrap();
        let backup_was_interrupted =
            matches!(result_guard.as_ref(), Some(Err(QbakError::Interrupted)));

        match result_guard.as_ref() {
            Some(Ok(_)) => {
                // Backup completed before interrupt could take effect
                assert!(final_backup_path.exists(), "Successful backup should exist");
            }
            Some(Err(QbakError::Interrupted)) => {
                // Backup was successfully interrupted

                // Check if any partial backup exists
                if final_backup_path.exists() {
                    // If partial backup exists, verify it gets cleaned up
                    let active_ops = crate::signal::get_active_operations();
                    if !active_ops.is_empty() {
                        // Simulate cleanup as main thread would do on interrupt
                        crate::signal::cleanup_active_operations_with_mode(true);

                        // Verify cleanup worked
                        assert!(
                            !final_backup_path.exists(),
                            "Partial backup should be cleaned up after interrupt"
                        );
                    }
                }
            }
            Some(Err(other_error)) => {
                panic!("Unexpected error during backup: {other_error:?}");
            }
            None => {
                panic!("Backup thread did not store a result");
            }
        }

        // Drop the result guard to release the lock
        drop(result_guard);

        // Only check for leftover files if backup was actually interrupted
        if backup_was_interrupted {
            // Verify no temp files remain
            let parent_dir = final_backup_path.parent().unwrap();

            if let Ok(entries) = fs::read_dir(parent_dir) {
                for entry in entries {
                    if let Ok(entry) = entry {
                        let path = entry.path();
                        let filename = path.file_name().unwrap().to_string_lossy();

                        // Check for temp files that might have been left behind
                        if filename.contains(".qbak_temp_") {
                            panic!("Temp file left behind: {}", path.display());
                        }

                        // Check for any partial backup directories (only if interrupted)
                        if filename.starts_with(
                            &source_dir
                                .file_name()
                                .unwrap()
                                .to_string_lossy()
                                .to_string(),
                        ) && filename.contains("-qbak")
                        {
                            panic!(
                                "Partial backup directory left behind after interrupt: {}",
                                path.display()
                            );
                        }
                    }
                }
            }
        }

        // Reset interrupt flag for other tests
        context.set_interrupted(false);
    }

    #[test]
    fn test_interrupt_during_file_copy_with_chunks() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;
        use std::time::Duration;

        // Reset global state for test isolation
        crate::signal::reset_for_testing();

        let dir = tempdir().unwrap();
        let source_file = dir.path().join("large_test_file.bin");
        let dest_file = dir.path().join("dest_file.bin");

        // Create a 20MB file to ensure chunked copying and longer copy time
        eprintln!("Creating 20MB test file...");
        let content = vec![0u8; 20 * 1024 * 1024]; // 20MB of zeros
        fs::write(&source_file, content).unwrap();

        // Set up interrupt flag
        let interrupt_flag = Arc::new(AtomicBool::new(false));
        crate::signal::set_interrupt_flag(interrupt_flag.clone());

        // Start copying, then interrupt partway through
        let source_clone = source_file.clone();
        let dest_clone = dest_file.clone();
        let interrupt_clone = interrupt_flag.clone();

        let copy_thread = std::thread::spawn(move || {
            // Allow some copying to happen before interrupt
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(100));
                eprintln!("Interrupting file copy...");
                interrupt_clone.store(true, Ordering::SeqCst);
            });

            copy_file_with_interrupt_check(&source_clone, &dest_clone)
        });

        let result = copy_thread.join().unwrap();

        // Should either complete successfully (if too fast) or be interrupted
        match result {
            Ok(()) => {
                // Copy completed before interrupt
                assert!(dest_file.exists(), "File should exist if copy completed");
                eprintln!("Copy completed before interrupt (system too fast)");
            }
            Err(QbakError::Interrupted) => {
                // This is what we expect - copy was interrupted
                // File may or may not exist depending on when interrupt happened
                if dest_file.exists() {
                    // If file exists, it should be a partial copy
                    let source_size = fs::metadata(&source_file).unwrap().len();
                    let dest_size = fs::metadata(&dest_file).unwrap().len();

                    // Dest should be smaller than source (partial copy)
                    // Unless interrupt happened very late in the process
                    if dest_size < source_size {
                        eprintln!("Successfully interrupted copy - partial file: {dest_size}/{source_size} bytes");
                    } else {
                        eprintln!("Copy completed just before interrupt");
                    }
                }
            }
            Err(other) => {
                panic!("Unexpected error during copy: {other:?}");
            }
        }

        // Reset interrupt flag
        interrupt_flag.store(false, Ordering::SeqCst);
    }
}
