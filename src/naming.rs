use crate::config::Config;
use crate::error::QbakError;
use crate::Result;
use chrono::{DateTime, Utc};
use std::path::{Path, PathBuf};

/// Generate a backup filename based on the source path and configuration
pub fn generate_backup_name(source: &Path, config: &Config) -> Result<PathBuf> {
    let timestamp = Utc::now();
    let timestamp_str = format_timestamp(&timestamp, &config.timestamp_format);

    let source_name = source
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| QbakError::validation("Invalid source filename"))?;

    // Split filename into stem and extension
    let (stem, extension) = split_filename(source_name);

    // Create backup filename
    let backup_name = if extension.is_empty() {
        let suffix = &config.backup_suffix;
        format!("{stem}-{timestamp_str}-{suffix}")
    } else {
        format!(
            "{stem}-{timestamp_str}-{}.{extension}",
            config.backup_suffix
        )
    };

    // Validate the generated filename
    validate_filename_length(&backup_name, config.max_filename_length)?;
    validate_filesystem_chars(&backup_name)?;

    // Get the parent directory
    let parent = source.parent().unwrap_or(Path::new("."));
    let backup_path = parent.join(&backup_name);

    Ok(backup_path)
}

/// Resolve filename collisions by adding a counter
pub fn resolve_collision(base_path: &Path) -> Result<PathBuf> {
    if !base_path.exists() {
        return Ok(base_path.to_path_buf());
    }

    let parent = base_path.parent().unwrap_or(Path::new("."));
    let filename = base_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| QbakError::validation("Invalid backup filename"))?;

    // Split the filename to insert counter before extension
    let (stem, extension) = split_filename(filename);

    // Try adding counters until we find an available name
    for counter in 1..=9999 {
        let new_name = if extension.is_empty() {
            format!("{stem}-{counter}")
        } else {
            format!("{stem}-{counter}.{extension}")
        };

        let new_path = parent.join(&new_name);
        if !new_path.exists() {
            return Ok(new_path);
        }
    }

    Err(QbakError::validation("Too many backup collisions (>9999)"))
}

/// Format timestamp according to the specified format
fn format_timestamp(timestamp: &DateTime<Utc>, format: &str) -> String {
    match format {
        "YYYYMMDDTHHMMSS" => timestamp.format("%Y%m%dT%H%M%S").to_string(),
        _ => {
            // For now, we only support the default format
            // In the future, we could add support for custom formats
            timestamp.format("%Y%m%dT%H%M%S").to_string()
        }
    }
}

/// Split filename into stem and extension
fn split_filename(filename: &str) -> (&str, &str) {
    if let Some(dot_pos) = filename.rfind('.') {
        // Only split if the dot is not at the beginning or end
        if dot_pos > 0 && dot_pos < filename.len() - 1 {
            return (&filename[..dot_pos], &filename[dot_pos + 1..]);
        } else if dot_pos == filename.len() - 1 {
            // Filename ends with dot - treat as no extension
            return (&filename[..dot_pos], "");
        }
    }
    (filename, "")
}

/// Validate that the filename doesn't exceed the maximum length
fn validate_filename_length(filename: &str, max_length: usize) -> Result<()> {
    if filename.len() > max_length {
        return Err(QbakError::FilenameTooLong {
            length: filename.len(),
            max: max_length,
        });
    }
    Ok(())
}

/// Validate that the filename doesn't contain problematic characters
fn validate_filesystem_chars(filename: &str) -> Result<()> {
    // Characters that are problematic on Windows
    const INVALID_CHARS: &[char] = &['<', '>', ':', '"', '|', '?', '*'];

    // Check for Windows reserved names
    const RESERVED_NAMES: &[&str] = &[
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
        "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];

    // Check for invalid characters
    let invalid_chars: Vec<char> = filename
        .chars()
        .filter(|c| INVALID_CHARS.contains(c))
        .collect();
    if !invalid_chars.is_empty() {
        let chars_str: String = invalid_chars.into_iter().collect();
        return Err(QbakError::InvalidFilesystemChars { chars: chars_str });
    }

    // Check for reserved names (case-insensitive)
    let stem = split_filename(filename).0.to_uppercase();
    if RESERVED_NAMES.contains(&stem.as_str()) {
        return Err(QbakError::InvalidFilesystemChars {
            chars: format!("Reserved name: {stem}"),
        });
    }

    // Check for control characters
    if filename.chars().any(|c| c.is_control()) {
        return Err(QbakError::InvalidFilesystemChars {
            chars: "Control characters".to_string(),
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::default_config;
    use std::fs::File;
    use tempfile::tempdir;

    #[test]
    fn test_split_filename() {
        assert_eq!(split_filename("file.txt"), ("file", "txt"));
        assert_eq!(split_filename("file.tar.gz"), ("file.tar", "gz"));
        assert_eq!(split_filename("file"), ("file", ""));
        assert_eq!(split_filename(".hidden"), (".hidden", ""));
        assert_eq!(split_filename("file."), ("file", ""));
    }

    #[test]
    fn test_generate_backup_name() {
        let config = default_config();
        let source = Path::new("/tmp/test.txt");

        let backup_path = generate_backup_name(source, &config).unwrap();
        let backup_name = backup_path.file_name().unwrap().to_str().unwrap();

        // Should contain timestamp and suffix
        assert!(backup_name.contains("-qbak"));
        assert!(backup_name.ends_with(".txt"));
        assert!(backup_name.starts_with("test-"));
    }

    #[test]
    fn test_generate_backup_name_no_extension() {
        let config = default_config();
        let source = Path::new("/tmp/makefile");

        let backup_path = generate_backup_name(source, &config).unwrap();
        let backup_name = backup_path.file_name().unwrap().to_str().unwrap();

        // Should contain timestamp and suffix, no extension
        assert!(backup_name.contains("-qbak"));
        assert!(!backup_name.contains('.'));
        assert!(backup_name.starts_with("makefile-"));
    }

    #[test]
    fn test_resolve_collision() {
        let dir = tempdir().unwrap();
        let base_path = dir.path().join("test-20250101T120000-qbak.txt");

        // No collision - should return original path
        let resolved = resolve_collision(&base_path).unwrap();
        assert_eq!(resolved, base_path);

        // Create the file to force collision
        File::create(base_path.clone()).unwrap();
        let resolved = resolve_collision(&base_path).unwrap();
        assert_eq!(resolved, dir.path().join("test-20250101T120000-qbak-1.txt"));

        // Create that too
        File::create(resolved.clone()).unwrap();
        let resolved2 = resolve_collision(&base_path).unwrap();
        assert_eq!(
            resolved2,
            dir.path().join("test-20250101T120000-qbak-2.txt")
        );
    }

    #[test]
    fn test_validate_filesystem_chars() {
        // Valid filenames
        assert!(validate_filesystem_chars("normal-file.txt").is_ok());
        assert!(validate_filesystem_chars("file_with-chars.123").is_ok());

        // Invalid characters
        assert!(validate_filesystem_chars("file<test>.txt").is_err());
        assert!(validate_filesystem_chars("file:test.txt").is_err());
        assert!(validate_filesystem_chars("file|test.txt").is_err());

        // Reserved names
        assert!(validate_filesystem_chars("CON.txt").is_err());
        assert!(validate_filesystem_chars("con.txt").is_err());
        assert!(validate_filesystem_chars("COM1.txt").is_err());
    }

    #[test]
    fn test_filename_length_validation() {
        let short_name = "test.txt";
        let long_name = "a".repeat(300);

        assert!(validate_filename_length(short_name, 255).is_ok());
        assert!(validate_filename_length(&long_name, 255).is_err());
    }
}
