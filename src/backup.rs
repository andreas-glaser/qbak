use crate::config::Config;
use crate::error::QbakError;
use crate::naming::{generate_backup_name, resolve_collision};
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

    // Calculate size for reporting
    let file_size = calculate_size(source)?;

    // Perform atomic copy
    let temp_path = create_temp_backup_path(&final_backup_path)?;

    // Copy the file
    fs::copy(source, &temp_path)?;

    // Copy metadata if configured
    if config.preserve_permissions {
        copy_permissions(source, &temp_path)?;
        copy_timestamps(source, &temp_path)?;
    }

    // Atomic rename
    fs::rename(&temp_path, &final_backup_path)?;

    let duration = start_time.elapsed();

    Ok(BackupResult {
        source_path: source.to_path_buf(),
        backup_path: final_backup_path,
        files_processed: 1,
        total_size: file_size,
        duration,
    })
}

/// Backup a directory recursively
pub fn backup_directory(source: &Path, config: &Config) -> Result<BackupResult> {
    let start_time = Instant::now();

    // Validate source
    validate_source(source)?;

    if !source.is_dir() {
        return Err(QbakError::validation("Source is not a directory"));
    }

    // Generate backup name
    let backup_path = generate_backup_name(source, config)?;
    let final_backup_path = resolve_collision(&backup_path)?;

    // Create backup directory
    fs::create_dir_all(&final_backup_path)?;

    // Copy directory contents
    let mut result = BackupResult::new(source.to_path_buf(), final_backup_path.clone());
    copy_directory_contents(source, &final_backup_path, config, &mut result)?;

    // Set directory permissions if configured
    if config.preserve_permissions {
        copy_permissions(source, &final_backup_path)?;
        copy_timestamps(source, &final_backup_path)?;
    }

    result.duration = start_time.elapsed();
    Ok(result)
}

/// Copy directory contents recursively
fn copy_directory_contents(
    source_dir: &Path,
    backup_dir: &Path,
    config: &Config,
    result: &mut BackupResult,
) -> Result<()> {
    for entry in fs::read_dir(source_dir)? {
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
        } else if metadata.is_dir() {
            // Create directory and recurse
            fs::create_dir_all(&backup_path)?;
            copy_directory_contents(&source_path, &backup_path, config, result)?;

            // Set directory permissions
            if config.preserve_permissions {
                copy_permissions(&source_path, &backup_path)?;
                copy_timestamps(&source_path, &backup_path)?;
            }
        } else if metadata.is_symlink() {
            // Handle symlinks
            handle_symlink(&source_path, &backup_path, config, result)?;
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

    // Copy file
    fs::copy(source, &temp_path)?;

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

/// Handle symlink based on configuration
fn handle_symlink(
    source: &Path,
    backup: &Path,
    config: &Config,
    result: &mut BackupResult,
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
                copy_directory_contents(&resolved_target, backup, config, result)?;
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

    let temp_name = format!(".qbak_temp_{}_{}", std::process::id(), filename);
    Ok(parent.join(temp_name))
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
        let result = backup_directory(&source_dir, &config).unwrap();

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
        let result = backup_directory(&file_path, &config);

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
        let result = backup_directory(&source_dir, &config).unwrap();
        assert_eq!(result.files_processed, 2);
        assert!(result.backup_path.join(".hidden.txt").exists());

        // Clean up for next test
        fs::remove_dir_all(&result.backup_path).unwrap();

        // Test with include_hidden = false
        config.include_hidden = false;
        let result = backup_directory(&source_dir, &config).unwrap();
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
        let result = backup_directory(&source_dir, &config).unwrap();

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
        fs::write(&deep_path.join("deep.txt"), "deep").unwrap();

        let config = default_config();
        let result = backup_directory(&source_dir, &config).unwrap();

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
        assert!(summary.contains("Created backup: backup.txt"));
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
        assert!(summary.contains("Created backup: backup_dir"));
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

            let result = backup_directory(&source_dir, &config).unwrap();

            // Should process both the regular file and the symlink target
            assert_eq!(result.files_processed, 2);
            assert!(result.backup_path.join("regular.txt").exists());
            assert!(result.backup_path.join("link.txt").exists());

            // Clean up
            fs::remove_dir_all(&result.backup_path).unwrap();

            // Test with follow_symlinks = false
            config.follow_symlinks = false;
            let result = backup_directory(&source_dir, &config).unwrap();

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
        let result = backup_directory(&source_dir, &config).unwrap();

        assert_eq!(result.files_processed, 4);
        assert_eq!(result.total_size, 23); // 0 + 1 + 11 + 11 = 23 bytes

        // Verify all files were backed up
        assert!(result.backup_path.join("empty.txt").exists());
        assert!(result.backup_path.join("small.txt").exists());
        assert!(result.backup_path.join("medium.txt").exists());
        assert!(result.backup_path.join("sub").join("sub_file.txt").exists());
    }
}
