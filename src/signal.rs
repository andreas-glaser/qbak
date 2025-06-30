use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

/// Global registry of active backup operations that need cleanup on interruption
static ACTIVE_OPERATIONS: OnceLock<Mutex<HashSet<PathBuf>>> = OnceLock::new();

/// Get or initialize the active operations registry
fn get_operations_mutex() -> &'static Mutex<HashSet<PathBuf>> {
    ACTIVE_OPERATIONS.get_or_init(|| Mutex::new(HashSet::new()))
}

/// RAII guard that ensures cleanup of backup operation on drop
pub struct BackupOperationGuard {
    backup_path: PathBuf,
    registered: bool,
    completed: bool,
}

impl BackupOperationGuard {
    /// Register a new backup operation for cleanup tracking
    pub fn new(backup_path: PathBuf) -> Self {
        let mut guard = Self {
            backup_path: backup_path.clone(),
            registered: false,
            completed: false,
        };

        if let Ok(mut operations) = get_operations_mutex().lock() {
            operations.insert(backup_path);
            guard.registered = true;
        }

        guard
    }

    /// Mark the operation as completed (prevents cleanup on drop)
    pub fn complete(mut self) {
        if let Ok(mut operations) = get_operations_mutex().lock() {
            operations.remove(&self.backup_path);
            self.registered = false;
            self.completed = true;
        }
    }
}

impl Drop for BackupOperationGuard {
    fn drop(&mut self) {
        if self.registered && !self.completed {
            // Operation was interrupted or failed - remove from tracking but don't clean up files
            // The signal handler (cleanup_active_operations) will handle actual file cleanup
            if let Ok(mut operations) = get_operations_mutex().lock() {
                operations.remove(&self.backup_path);
            }
        }
        // If completed, already removed from tracking in complete()
    }
}

/// Get a snapshot of currently active operations (for signal handler)
pub fn get_active_operations() -> Vec<PathBuf> {
    get_operations_mutex()
        .lock()
        .map(|operations| operations.iter().cloned().collect())
        .unwrap_or_default()
}

/// Clean up all active backup operations
pub fn cleanup_active_operations() {
    cleanup_active_operations_with_mode(false);
}

/// Clean up all active backup operations with optional silent mode
pub fn cleanup_active_operations_with_mode(silent: bool) {
    let active_ops = get_active_operations();

    for backup_path in active_ops {
        if backup_path.exists() {
            let cleanup_result = if backup_path.is_dir() {
                std::fs::remove_dir_all(&backup_path)
            } else {
                std::fs::remove_file(&backup_path)
            };

            // Log cleanup attempt (unless silent)
            if !silent {
                if cleanup_result.is_ok() {
                    eprintln!("Cleaned up incomplete backup: {}", backup_path.display());
                } else {
                    eprintln!(
                        "Warning: Could not clean up incomplete backup: {}",
                        backup_path.display()
                    );
                }
            }
        }
    }

    // Clear the tracking registry
    if let Ok(mut operations) = get_operations_mutex().lock() {
        operations.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_backup_operation_guard_normal_completion() {
        let dir = tempdir().unwrap();
        let backup_path = dir.path().join("test-backup");
        fs::create_dir(&backup_path).unwrap();

        // Create guard and verify it's tracked
        let guard = BackupOperationGuard::new(backup_path.clone());
        let active_ops = get_active_operations();
        assert!(active_ops.contains(&backup_path));

        // Complete the operation
        guard.complete();

        // Verify it's no longer tracked
        let active_ops = get_active_operations();
        assert!(!active_ops.contains(&backup_path));

        // Backup should still exist
        assert!(backup_path.exists());
    }

    #[test]
    fn test_backup_operation_guard_interrupted() {
        let dir = tempdir().unwrap();
        let backup_path = dir.path().join("test-backup");
        fs::create_dir(&backup_path).unwrap();

        {
            // Create guard and verify it's tracked
            let _guard = BackupOperationGuard::new(backup_path.clone());
            let active_ops = get_active_operations();
            assert!(active_ops.contains(&backup_path));
        } // Guard is dropped here without calling complete()

        // Verify it's no longer tracked (cleaned up)
        let active_ops = get_active_operations();
        assert!(!active_ops.contains(&backup_path));
    }

    #[test]
    fn test_cleanup_active_operations() {
        let dir = tempdir().unwrap();
        let backup_path1 = dir.path().join("backup1");
        let backup_path2 = dir.path().join("backup2");

        fs::create_dir(&backup_path1).unwrap();
        fs::write(&backup_path2, "content").unwrap();

        // Register operations
        let _guard1 = BackupOperationGuard::new(backup_path1.clone());
        let _guard2 = BackupOperationGuard::new(backup_path2.clone());

        // Verify they're tracked
        let active_ops = get_active_operations();
        assert!(active_ops.contains(&backup_path1));
        assert!(active_ops.contains(&backup_path2));

        // Verify backups exist
        assert!(backup_path1.exists());
        assert!(backup_path2.exists());

        // Cleanup all active operations
        cleanup_active_operations();

        // Verify they're no longer tracked
        let active_ops = get_active_operations();
        assert!(!active_ops.contains(&backup_path1));
        assert!(!active_ops.contains(&backup_path2));

        // Verify backups were removed
        assert!(!backup_path1.exists());
        assert!(!backup_path2.exists());
    }

    #[test]
    fn test_cleanup_nonexistent_operations() {
        let dir = tempdir().unwrap();
        let backup_path = dir.path().join("nonexistent-backup");

        // Register operation for path that doesn't exist
        let _guard = BackupOperationGuard::new(backup_path.clone());

        // Cleanup should handle gracefully
        cleanup_active_operations();

        // Should not panic or error
        let active_ops = get_active_operations();
        assert!(!active_ops.contains(&backup_path));
    }
}
