use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
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

impl QbakError {
    /// Create a configuration error with a custom message
    pub fn config(message: impl Into<String>) -> Self {
        Self::Config {
            message: message.into(),
        }
    }

    /// Create a validation error with a custom message
    pub fn validation(message: impl Into<String>) -> Self {
        Self::Validation {
            message: message.into(),
        }
    }

    /// Check if this error is recoverable (operation can continue)
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            QbakError::SourceNotFound { .. }
                | QbakError::PermissionDenied { .. }
                | QbakError::Validation { .. }
        )
    }

    /// Get the exit code for this error
    pub fn exit_code(&self) -> i32 {
        match self {
            QbakError::Interrupted => 130,
            QbakError::Validation { .. } => 2,
            QbakError::Config { .. } => 2,
            _ => 1,
        }
    }

    /// Provide helpful suggestions for resolving the error
    pub fn suggestions(&self) -> Vec<String> {
        match self {
            QbakError::FilenameTooLong { .. } => vec![
                "Rename the source file to be shorter".to_string(),
                "Move to a directory with a shorter path".to_string(),
                "Use a shorter backup suffix in config".to_string(),
            ],
            QbakError::InvalidFilesystemChars { chars } => vec![
                format!("Rename file to remove problematic characters: {}", chars),
                "Use a different filesystem that supports these characters".to_string(),
            ],
            QbakError::InsufficientSpace { .. } => vec![
                "Free up disk space".to_string(),
                "Choose a different backup location".to_string(),
                "Remove old backup files".to_string(),
            ],
            QbakError::PermissionDenied { .. } => vec![
                "Check file permissions".to_string(),
                "Run with appropriate privileges".to_string(),
                "Ensure parent directory is writable".to_string(),
            ],
            _ => vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    #[test]
    fn test_error_creation() {
        let path = PathBuf::from("/test/path");

        let source_not_found = QbakError::SourceNotFound { path: path.clone() };
        assert!(format!("{}", source_not_found).contains("/test/path"));

        let filename_too_long = QbakError::FilenameTooLong {
            length: 300,
            max: 255,
        };
        assert!(format!("{}", filename_too_long).contains("300"));
        assert!(format!("{}", filename_too_long).contains("255"));

        let insufficient_space = QbakError::InsufficientSpace {
            needed: 1000,
            available: 500,
        };
        assert!(format!("{}", insufficient_space).contains("1000"));
        assert!(format!("{}", insufficient_space).contains("500"));
    }

    #[test]
    fn test_config_and_validation_constructors() {
        let config_error = QbakError::config("Test config error");
        match config_error {
            QbakError::Config { message } => assert_eq!(message, "Test config error"),
            _ => panic!("Expected Config error"),
        }

        let validation_error = QbakError::validation("Test validation error");
        match validation_error {
            QbakError::Validation { message } => assert_eq!(message, "Test validation error"),
            _ => panic!("Expected Validation error"),
        }
    }

    #[test]
    fn test_is_recoverable() {
        let path = PathBuf::from("/test");

        // Recoverable errors
        assert!(QbakError::SourceNotFound { path: path.clone() }.is_recoverable());
        assert!(QbakError::PermissionDenied { path: path.clone() }.is_recoverable());
        assert!(QbakError::validation("test").is_recoverable());

        // Non-recoverable errors
        assert!(!QbakError::Interrupted.is_recoverable());
        assert!(!QbakError::config("test").is_recoverable());
        assert!(!QbakError::FilenameTooLong {
            length: 300,
            max: 255
        }
        .is_recoverable());
        assert!(!QbakError::InsufficientSpace {
            needed: 1000,
            available: 500
        }
        .is_recoverable());
    }

    #[test]
    fn test_exit_codes() {
        assert_eq!(QbakError::Interrupted.exit_code(), 130);
        assert_eq!(QbakError::validation("test").exit_code(), 2);
        assert_eq!(QbakError::config("test").exit_code(), 2);

        let path = PathBuf::from("/test");
        assert_eq!(QbakError::SourceNotFound { path }.exit_code(), 1);
        assert_eq!(
            QbakError::FilenameTooLong {
                length: 300,
                max: 255
            }
            .exit_code(),
            1
        );
    }

    #[test]
    fn test_suggestions() {
        // FilenameTooLong suggestions
        let filename_error = QbakError::FilenameTooLong {
            length: 300,
            max: 255,
        };
        let suggestions = filename_error.suggestions();
        assert!(!suggestions.is_empty());
        assert!(suggestions.iter().any(|s| s.contains("shorter")));

        // InvalidFilesystemChars suggestions
        let chars_error = QbakError::InvalidFilesystemChars {
            chars: "<>".to_string(),
        };
        let suggestions = chars_error.suggestions();
        assert!(!suggestions.is_empty());
        assert!(suggestions.iter().any(|s| s.contains("<>")));

        // InsufficientSpace suggestions
        let space_error = QbakError::InsufficientSpace {
            needed: 1000,
            available: 500,
        };
        let suggestions = space_error.suggestions();
        assert!(!suggestions.is_empty());
        assert!(suggestions.iter().any(|s| s.contains("disk space")));

        // PermissionDenied suggestions
        let path = PathBuf::from("/test");
        let perm_error = QbakError::PermissionDenied { path };
        let suggestions = perm_error.suggestions();
        assert!(!suggestions.is_empty());
        assert!(suggestions.iter().any(|s| s.contains("permission")));

        // Error with no suggestions
        let no_suggestions_error = QbakError::Interrupted;
        assert!(no_suggestions_error.suggestions().is_empty());
    }

    #[test]
    fn test_io_error_conversion() {
        let io_error = io::Error::new(io::ErrorKind::NotFound, "File not found");
        let qbak_error: QbakError = io_error.into();

        match qbak_error {
            QbakError::Io(_) => (), // Expected
            _ => panic!("Expected IO error conversion"),
        }
    }

    #[test]
    fn test_error_display() {
        let path = PathBuf::from("/test/file.txt");

        let errors = vec![
            QbakError::SourceNotFound { path: path.clone() },
            QbakError::FilenameTooLong {
                length: 300,
                max: 255,
            },
            QbakError::InsufficientSpace {
                needed: 1000,
                available: 500,
            },
            QbakError::PermissionDenied { path: path.clone() },
            QbakError::InvalidFilesystemChars {
                chars: "<>".to_string(),
            },
            QbakError::SymlinkLoop { path: path.clone() },
            QbakError::BackupExists { path: path.clone() },
            QbakError::PathTraversal { path },
            QbakError::config("Config test"),
            QbakError::Interrupted,
            QbakError::validation("Validation test"),
        ];

        for error in errors {
            let display = format!("{}", error);
            assert!(!display.is_empty(), "Error display should not be empty");
        }
    }
}
