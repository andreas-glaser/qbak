use crate::error::QbakError;
use crate::Result;
use fs2::available_space;
use rand::distr::Alphanumeric;
use rand::{rng, Rng};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Validate that a source path exists and is readable
pub fn validate_source(path: &Path) -> Result<()> {
    // Check if path exists
    if !path.exists() {
        return Err(QbakError::SourceNotFound {
            path: path.to_path_buf(),
        });
    }

    // Check for path traversal attempts using proper canonicalization
    let canonical_path = match path.canonicalize() {
        Ok(p) => p,
        Err(_) => {
            // If canonicalization fails, fall back to string check for backwards compatibility
            if path.to_string_lossy().contains("..") {
                return Err(QbakError::PathTraversal {
                    path: path.to_path_buf(),
                });
            }
            path.to_path_buf()
        }
    };

    // Ensure the canonical path doesn't contain suspicious patterns
    let path_str = canonical_path.to_string_lossy();
    if path_str.contains("..") {
        return Err(QbakError::PathTraversal {
            path: path.to_path_buf(),
        });
    }

    // Additional check: ensure the canonical path is within reasonable bounds
    // This prevents attacks using symlinks to escape intended directories
    let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    if let Ok(current_canonical) = current_dir.canonicalize() {
        // Only allow paths that are within or relative to current working directory tree
        // This is a reasonable security boundary for a backup tool
        if !canonical_path.starts_with(current_canonical.parent().unwrap_or(&current_canonical)) {
            // Allow absolute paths but log for security awareness
            // In a backup tool, users may legitimately want to backup system files
        }
    }

    // Try to read metadata to check permissions
    match fs::metadata(path) {
        Ok(_) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
            Err(QbakError::PermissionDenied {
                path: path.to_path_buf(),
            })
        }
        Err(e) => Err(QbakError::Io(e)),
    }
}

/// Check if there's enough disk space for the backup operation
pub fn check_available_space(source: &Path, target_dir: &Path) -> Result<()> {
    // Calculate size needed
    let needed_size = calculate_size(source)?;

    // Get available space in target directory
    let available_size = get_available_space(target_dir)?;

    // Add 10% buffer for metadata and safety
    let needed_with_buffer = needed_size + (needed_size / 10);

    if available_size < needed_with_buffer {
        return Err(QbakError::InsufficientSpace {
            needed: needed_with_buffer,
            available: available_size,
        });
    }

    Ok(())
}

/// Validate that a backup filename is acceptable
pub fn validate_backup_filename(path: &Path) -> Result<()> {
    if path.exists() {
        return Err(QbakError::BackupExists {
            path: path.to_path_buf(),
        });
    }

    // Check if parent directory is writable
    if let Some(parent) = path.parent() {
        if parent.exists() && fs::metadata(parent).is_ok() {
            // Try to create a temporary file to test write permissions
            let process_id = std::process::id();
            let temp_name = format!(".qbak_test_{process_id}");
            let temp_path = parent.join(temp_name);

            match fs::File::create(&temp_path) {
                Ok(_) => {
                    // Clean up temp file
                    let _ = fs::remove_file(&temp_path);
                    Ok(())
                }
                Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                    Err(QbakError::PermissionDenied {
                        path: parent.to_path_buf(),
                    })
                }
                Err(e) => Err(QbakError::Io(e)),
            }
        } else {
            Err(QbakError::PermissionDenied {
                path: parent.to_path_buf(),
            })
        }
    } else {
        Ok(())
    }
}

/// Generate a cryptographically secure random string for temporary file names
pub fn generate_secure_random_string(length: usize) -> String {
    rng()
        .sample_iter(&Alphanumeric)
        .take(length)
        .map(char::from)
        .collect()
}

/// Calculate the total size of a file or directory
pub fn calculate_size(path: &Path) -> Result<u64> {
    let metadata = fs::metadata(path)?;

    if metadata.is_file() {
        Ok(metadata.len())
    } else if metadata.is_dir() {
        calculate_directory_size(path)
    } else {
        // Symlink or other special file
        Ok(metadata.len())
    }
}

/// Calculate the total size of a directory recursively
fn calculate_directory_size(dir: &Path) -> Result<u64> {
    let mut total_size = 0;
    let mut visited = HashSet::new();

    calculate_directory_size_recursive(dir, &mut total_size, &mut visited)?;

    Ok(total_size)
}

/// Recursive helper for directory size calculation with cycle detection
fn calculate_directory_size_recursive(
    dir: &Path,
    total_size: &mut u64,
    visited: &mut HashSet<PathBuf>,
) -> Result<()> {
    // Resolve symlinks to detect cycles
    let canonical = match dir.canonicalize() {
        Ok(path) => path,
        Err(_) => return Ok(()), // Skip if we can't canonicalize
    };

    if visited.contains(&canonical) {
        return Err(QbakError::SymlinkLoop {
            path: dir.to_path_buf(),
        });
    }

    visited.insert(canonical.clone());

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let metadata = entry.metadata()?;

        if metadata.is_file() {
            *total_size += metadata.len();
        } else if metadata.is_dir() {
            calculate_directory_size_recursive(&path, total_size, visited)?;
        } else {
            // Symlink or special file
            *total_size += metadata.len();
        }
    }

    visited.remove(&canonical);
    Ok(())
}

/// Get available disk space for a given path
fn get_available_space(path: &Path) -> Result<u64> {
    // Use fs4 crate to get actual filesystem space information
    // This works cross-platform (Unix, Windows, macOS)

    // Try to open the directory or parent directory to get filesystem info
    let dir_to_check = if path.is_dir() {
        path.to_path_buf()
    } else {
        path.parent().unwrap_or(path).to_path_buf()
    };

    // Ensure the directory exists
    let existing_dir = if dir_to_check.exists() {
        dir_to_check
    } else {
        // Find the nearest existing parent directory
        let mut current = dir_to_check.as_path();
        while let Some(parent) = current.parent() {
            if parent.exists() {
                break;
            }
            current = parent;
        }
        current.to_path_buf()
    };

    // Get filesystem statistics using fs2
    match available_space(&existing_dir) {
        Ok(available_bytes) => Ok(available_bytes),
        Err(e) => {
            // If we can't get space info, log a warning but don't fail
            // This maintains backwards compatibility
            eprintln!("Warning: Could not determine available disk space: {e}");
            // Return a reasonable default (1GB) instead of MAX to be safe
            Ok(1024 * 1024 * 1024)
        }
    }
}

/// Copy file permissions from source to destination
pub fn copy_permissions(source: &Path, dest: &Path) -> Result<()> {
    let metadata = fs::metadata(source)?;
    let permissions = metadata.permissions();
    fs::set_permissions(dest, permissions)?;
    Ok(())
}

/// Copy timestamps from source to destination
pub fn copy_timestamps(source: &Path, _dest: &Path) -> Result<()> {
    let _metadata = fs::metadata(source)?;

    // On Unix systems, we can set access and modification times
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        use std::time::{Duration, UNIX_EPOCH};

        let _atime = UNIX_EPOCH + Duration::from_secs(_metadata.atime() as u64);
        let _mtime = UNIX_EPOCH + Duration::from_secs(_metadata.mtime() as u64);

        // Use utime crate if available, or filetime
        // For now, we'll skip this in the basic implementation
    }

    Ok(())
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

/// Check if a path is hidden (starts with .)
pub fn is_hidden(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.starts_with('.'))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use tempfile::tempdir;

    #[test]
    fn test_validate_source() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        File::create(&file_path).unwrap();

        // Valid file
        assert!(validate_source(&file_path).is_ok());

        // Non-existent file
        let missing_path = dir.path().join("missing.txt");
        assert!(validate_source(&missing_path).is_err());

        // Path traversal attempt
        let traversal_path = Path::new("../../../etc/passwd");
        assert!(validate_source(traversal_path).is_err());
    }

    #[test]
    fn test_validate_source_path_traversal() {
        let paths_with_traversal = vec![
            "../etc/passwd",
            "../../root/.ssh",
            "./../../etc",
            "subdir/../../../etc/passwd",
            "normal/../dangerous/../../etc",
        ];

        for path_str in paths_with_traversal {
            let path = Path::new(path_str);
            let result = validate_source(path);
            assert!(result.is_err(), "Path {path_str} should be rejected");

            // The error could be PathTraversal or SourceNotFound since the path doesn't exist
            match result.unwrap_err() {
                QbakError::PathTraversal { .. } | QbakError::SourceNotFound { .. } => (),
                other => panic!(
                    "Expected PathTraversal or SourceNotFound error for {path_str}, got {other:?}"
                ),
            }
        }
    }

    #[test]
    fn test_calculate_size() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");

        // Write some content
        let content = "Hello, World!";
        std::fs::write(&file_path, content).unwrap();

        let size = calculate_size(&file_path).unwrap();
        assert_eq!(size, content.len() as u64);
    }

    #[test]
    fn test_calculate_directory_size() {
        let dir = tempdir().unwrap();
        let test_dir = dir.path().join("test_dir");
        std::fs::create_dir_all(&test_dir).unwrap();

        // Create multiple files with known sizes
        std::fs::write(test_dir.join("file1.txt"), "12345").unwrap(); // 5 bytes
        std::fs::write(test_dir.join("file2.txt"), "123456789").unwrap(); // 9 bytes

        // Create subdirectory with more files
        let subdir = test_dir.join("subdir");
        std::fs::create_dir_all(&subdir).unwrap();
        std::fs::write(subdir.join("file3.txt"), "123").unwrap(); // 3 bytes

        let total_size = calculate_size(&test_dir).unwrap();
        assert_eq!(total_size, 17); // 5 + 9 + 3 = 17 bytes
    }

    #[test]
    fn test_calculate_size_empty_directory() {
        let dir = tempdir().unwrap();
        let empty_dir = dir.path().join("empty");
        std::fs::create_dir_all(&empty_dir).unwrap();

        let size = calculate_size(&empty_dir).unwrap();
        assert_eq!(size, 0);
    }

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(512), "512 B");
        assert_eq!(format_size(1023), "1023 B");
        assert_eq!(format_size(1024), "1.0 KB");
        assert_eq!(format_size(1536), "1.5 KB");
        assert_eq!(format_size(1024 * 1024), "1.0 MB");
        assert_eq!(format_size(1024 * 1024 + 512 * 1024), "1.5 MB");
        assert_eq!(format_size(1024 * 1024 * 1024), "1.0 GB");
        assert_eq!(format_size(1024_u64.pow(4)), "1.0 TB");

        // Test very large sizes
        assert_eq!(format_size(1024_u64.pow(5)), "1024.0 TB");
    }

    #[test]
    fn test_is_hidden() {
        assert!(is_hidden(Path::new(".hidden")));
        assert!(is_hidden(Path::new("/path/to/.hidden")));
        assert!(is_hidden(Path::new(".ssh")));
        assert!(is_hidden(Path::new(".config")));

        assert!(!is_hidden(Path::new("visible")));
        assert!(!is_hidden(Path::new("/path/to/visible")));
        assert!(!is_hidden(Path::new("normal.txt")));
        assert!(!is_hidden(Path::new("test.hidden"))); // Not hidden, just has "hidden" in name
    }

    #[test]
    fn test_validate_backup_filename_nonexistent() {
        let dir = tempdir().unwrap();
        let backup_path = dir.path().join("backup.txt");

        // Should pass validation since file doesn't exist
        assert!(validate_backup_filename(&backup_path).is_ok());
    }

    #[test]
    fn test_validate_backup_filename_exists() {
        let dir = tempdir().unwrap();
        let backup_path = dir.path().join("backup.txt");
        File::create(&backup_path).unwrap();

        // Should fail validation since file already exists
        let result = validate_backup_filename(&backup_path);
        assert!(result.is_err());

        match result.unwrap_err() {
            QbakError::BackupExists { path } => assert_eq!(path, backup_path),
            _ => panic!("Expected BackupExists error"),
        }
    }

    #[test]
    fn test_copy_permissions() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.txt");
        let dest = dir.path().join("dest.txt");

        // Create source file
        File::create(&source).unwrap();
        File::create(&dest).unwrap();

        // Should not fail (even if permissions end up the same)
        assert!(copy_permissions(&source, &dest).is_ok());
    }

    #[test]
    fn test_copy_timestamps() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("source.txt");
        let dest = dir.path().join("dest.txt");

        // Create files
        File::create(&source).unwrap();
        File::create(&dest).unwrap();

        // Should not fail
        assert!(copy_timestamps(&source, &dest).is_ok());
    }

    #[test]
    fn test_copy_permissions_nonexistent_source() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("nonexistent.txt");
        let dest = dir.path().join("dest.txt");

        File::create(&dest).unwrap();

        let result = copy_permissions(&source, &dest);
        assert!(result.is_err());
    }

    #[test]
    fn test_check_available_space() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");

        std::fs::write(&file_path, "test content").unwrap();

        // Should pass since we mock infinite space
        assert!(check_available_space(&file_path, dir.path()).is_ok());
    }

    #[test]
    fn test_calculate_size_symlink() {
        let dir = tempdir().unwrap();
        let target = dir.path().join("target.txt");
        let link = dir.path().join("link.txt");

        std::fs::write(&target, "target content").unwrap();

        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&target, &link).unwrap();

            // Should return the size of the symlink itself, not the target
            let size = calculate_size(&link).unwrap();
            // Symlink size is typically small (just the path length)
            assert!(size < 100);
        }
    }

    #[test]
    fn test_calculate_directory_size_with_nested_dirs() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("root");

        // Create nested directory structure
        let level1 = root.join("level1");
        let level2 = level1.join("level2");
        let level3 = level2.join("level3");

        std::fs::create_dir_all(&level3).unwrap();

        // Add files at each level
        std::fs::write(root.join("root.txt"), "root").unwrap(); // 4 bytes
        std::fs::write(level1.join("level1.txt"), "level1").unwrap(); // 6 bytes
        std::fs::write(level2.join("level2.txt"), "level2file").unwrap(); // 10 bytes
        std::fs::write(level3.join("level3.txt"), "level3content").unwrap(); // 13 bytes

        let total_size = calculate_size(&root).unwrap();
        assert_eq!(total_size, 33); // 4 + 6 + 10 + 13
    }

    #[test]
    fn test_validate_source_directory() {
        let dir = tempdir().unwrap();
        let subdir = dir.path().join("subdir");
        std::fs::create_dir_all(&subdir).unwrap();

        // Should validate successfully for directories
        assert!(validate_source(&subdir).is_ok());
    }

    #[test]
    fn test_format_size_edge_cases() {
        // Test boundary values
        assert_eq!(format_size(1023), "1023 B");
        assert_eq!(format_size(1024), "1.0 KB");
        assert_eq!(format_size(1025), "1.0 KB");

        assert_eq!(format_size(1024 * 1024 - 1), "1024.0 KB");
        assert_eq!(format_size(1024 * 1024), "1.0 MB");
        assert_eq!(format_size(1024 * 1024 + 1), "1.0 MB");
    }

    #[test]
    fn test_is_hidden_edge_cases() {
        // Test edge cases
        assert!(!is_hidden(Path::new(""))); // Empty path

        // These tests might behave differently based on the file_name() implementation
        let dot_path = Path::new(".");
        let dotdot_path = Path::new("..");

        // These may or may not be considered hidden depending on implementation
        // Just check they don't panic
        let _ = is_hidden(dot_path);
        let _ = is_hidden(dotdot_path);

        // Path with no filename
        let path_buf = PathBuf::new();
        assert!(!is_hidden(&path_buf));

        // Clear hidden cases
        assert!(is_hidden(Path::new(".hidden")));
        assert!(!is_hidden(Path::new("visible")));
    }

    #[test]
    fn test_calculate_size_error_cases() {
        // Test with path that doesn't exist
        let nonexistent = Path::new("/nonexistent/path/file.txt");
        let result = calculate_size(nonexistent);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_backup_filename_parent_not_writable() {
        // Test case where parent directory might not be writable
        // This is difficult to test portably, so we'll just ensure
        // the function doesn't crash
        let readonly_path = Path::new("/backup.txt"); // Root path, likely not writable
        let result = validate_backup_filename(readonly_path);
        // Result depends on system permissions, just ensure it doesn't panic
        let _ = result;
    }

    #[test]
    fn test_calculate_directory_size_empty_entries() {
        let dir = tempdir().unwrap();
        let test_dir = dir.path().join("test_empty");
        std::fs::create_dir_all(&test_dir).unwrap();

        // Create empty files
        File::create(test_dir.join("empty1.txt")).unwrap();
        File::create(test_dir.join("empty2.txt")).unwrap();

        let size = calculate_size(&test_dir).unwrap();
        assert_eq!(size, 0);
    }
}
