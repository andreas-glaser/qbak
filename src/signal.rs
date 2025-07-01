use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// Backup context that manages interrupt state and active operations for a backup session
#[derive(Clone)]
pub struct BackupContext {
    interrupt_flag: Arc<AtomicBool>,
    active_operations: Arc<Mutex<HashSet<PathBuf>>>,
}

impl BackupContext {
    /// Create a new backup context
    pub fn new() -> Self {
        Self {
            interrupt_flag: Arc::new(AtomicBool::new(false)),
            active_operations: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    /// Get the interrupt flag for signal handler setup
    pub fn interrupt_flag(&self) -> Arc<AtomicBool> {
        self.interrupt_flag.clone()
    }

    /// Check if an interrupt has been requested
    pub fn is_interrupted(&self) -> bool {
        self.interrupt_flag.load(Ordering::SeqCst)
    }

    /// Set interrupt state (mainly for testing)
    pub fn set_interrupted(&self, interrupted: bool) {
        self.interrupt_flag.store(interrupted, Ordering::SeqCst);
    }

    /// Register a backup operation for cleanup tracking
    pub fn register_operation(&self, backup_path: PathBuf) -> BackupOperationGuard {
        if let Ok(mut operations) = self.active_operations.lock() {
            operations.insert(backup_path.clone());
        }
        BackupOperationGuard::new(backup_path, self.clone())
    }

    /// Get a snapshot of currently active operations
    pub fn get_active_operations(&self) -> Vec<PathBuf> {
        self.active_operations
            .lock()
            .map(|operations| operations.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Clean up all active backup operations
    pub fn cleanup_active_operations(&self) {
        self.cleanup_active_operations_with_mode(false);
    }

    /// Clean up all active backup operations with optional silent mode
    pub fn cleanup_active_operations_with_mode(&self, silent: bool) {
        let active_ops = self.get_active_operations();

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
        if let Ok(mut operations) = self.active_operations.lock() {
            operations.clear();
        }
    }

    /// Remove an operation from tracking (internal use)
    fn remove_operation(&self, backup_path: &PathBuf) {
        if let Ok(mut operations) = self.active_operations.lock() {
            operations.remove(backup_path);
        }
    }
}

impl Default for BackupContext {
    fn default() -> Self {
        Self::new()
    }
}

// Global context for backward compatibility (will be removed in favor of instance-based)
static GLOBAL_CONTEXT: Mutex<Option<BackupContext>> = Mutex::new(None);

/// Set the global backup context
pub fn set_global_context(context: BackupContext) {
    if let Ok(mut global) = GLOBAL_CONTEXT.lock() {
        *global = Some(context);
    }
}

/// Get the global backup context
fn get_global_context() -> Option<BackupContext> {
    GLOBAL_CONTEXT.lock().ok().and_then(|global| global.clone())
}

/// Set the global interrupt flag
pub fn set_interrupt_flag(flag: Arc<AtomicBool>) {
    let context = BackupContext {
        interrupt_flag: flag,
        active_operations: Arc::new(Mutex::new(HashSet::new())),
    };
    set_global_context(context);
}

/// Check if an interrupt has been requested
pub fn is_interrupted() -> bool {
    get_global_context()
        .map(|ctx| ctx.is_interrupted())
        .unwrap_or(false)
}

/// Reset global state for testing
#[cfg(test)]
pub fn reset_for_testing() {
    if let Ok(mut global) = GLOBAL_CONTEXT.lock() {
        *global = Some(BackupContext::new());
    }
}

/// RAII guard that ensures cleanup of backup operation on drop
pub struct BackupOperationGuard {
    backup_path: PathBuf,
    context: BackupContext,
    registered: bool,
    completed: bool,
}

impl BackupOperationGuard {
    /// Create a new backup operation guard with the given context
    pub fn new(backup_path: PathBuf, context: BackupContext) -> Self {
        Self {
            backup_path,
            context,
            registered: true,
            completed: false,
        }
    }

    /// Mark the operation as completed (prevents cleanup on drop)
    pub fn complete(mut self) {
        self.context.remove_operation(&self.backup_path);
        self.registered = false;
        self.completed = true;
    }
}

impl Drop for BackupOperationGuard {
    fn drop(&mut self) {
        if self.registered && !self.completed {
            // If not interrupted, remove from tracking (normal failure/panic)
            // If interrupted, leave in tracking for cleanup by main thread
            if !self.context.is_interrupted() {
                self.context.remove_operation(&self.backup_path);
            }
        }
        // If completed, already removed from tracking in complete()
    }
}

/// Get a snapshot of currently active operations
pub fn get_active_operations() -> Vec<PathBuf> {
    get_global_context()
        .map(|ctx| ctx.get_active_operations())
        .unwrap_or_default()
}

/// Clean up all active backup operations
pub fn cleanup_active_operations() {
    cleanup_active_operations_with_mode(false);
}

/// Clean up all active backup operations with optional silent mode
pub fn cleanup_active_operations_with_mode(silent: bool) {
    if let Some(context) = get_global_context() {
        context.cleanup_active_operations_with_mode(silent);
    }
}

/// Create a BackupOperationGuard using the global context
pub fn create_backup_guard(backup_path: PathBuf) -> BackupOperationGuard {
    if let Some(context) = get_global_context() {
        context.register_operation(backup_path)
    } else {
        // Fallback if no global context is set
        let context = BackupContext::new();
        set_global_context(context.clone());
        context.register_operation(backup_path)
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

        // Create context and guard
        let context = BackupContext::new();
        let guard = context.register_operation(backup_path.clone());

        // Verify it's tracked
        let active_ops = context.get_active_operations();
        assert!(active_ops.contains(&backup_path));

        // Complete the operation
        guard.complete();

        // Verify it's no longer tracked
        let active_ops = context.get_active_operations();
        assert!(!active_ops.contains(&backup_path));

        // Backup should still exist
        assert!(backup_path.exists());
    }

    #[test]
    fn test_backup_operation_guard_interrupted() {
        let dir = tempdir().unwrap();
        let backup_path = dir.path().join("test-backup");
        fs::create_dir(&backup_path).unwrap();

        let context = BackupContext::new();

        {
            // Create guard and verify it's tracked
            let _guard = context.register_operation(backup_path.clone());
            let active_ops = context.get_active_operations();
            assert!(active_ops.contains(&backup_path));
        } // Guard is dropped here without calling complete()

        // Since we're not in an interrupted state, it should be removed from tracking
        let active_ops = context.get_active_operations();
        assert!(!active_ops.contains(&backup_path));
    }

    #[test]
    fn test_cleanup_active_operations() {
        let dir = tempdir().unwrap();
        let backup_path1 = dir.path().join("backup1");
        let backup_path2 = dir.path().join("backup2");

        fs::create_dir(&backup_path1).unwrap();
        fs::write(&backup_path2, "content").unwrap();

        let context = BackupContext::new();

        // Register operations
        let _guard1 = context.register_operation(backup_path1.clone());
        let _guard2 = context.register_operation(backup_path2.clone());

        // Verify they're tracked
        let active_ops = context.get_active_operations();
        assert!(active_ops.contains(&backup_path1));
        assert!(active_ops.contains(&backup_path2));

        // Verify backups exist
        assert!(backup_path1.exists());
        assert!(backup_path2.exists());

        // Cleanup all active operations
        context.cleanup_active_operations();

        // Verify they're no longer tracked
        let active_ops = context.get_active_operations();
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

        let context = BackupContext::new();

        // Register operation for path that doesn't exist
        let _guard = context.register_operation(backup_path.clone());

        // Cleanup should handle gracefully
        context.cleanup_active_operations();

        // Should not panic or error
        let active_ops = context.get_active_operations();
        assert!(!active_ops.contains(&backup_path));
    }

    #[test]
    fn test_interrupt_race_condition_fix() {
        // Test the race condition scenario that was causing the bug:
        // 1. Start backup operation
        // 2. Ctrl+C pressed (interrupt flag set)
        // 3. Backup operation detects interrupt and exits with error
        // 4. BackupOperationGuard drops
        // 5. Signal handler tries to clean up
        // 6. Verify partial backup is cleaned up

        let dir = tempdir().unwrap();
        let backup_path = dir.path().join("test-backup-interrupted");
        fs::create_dir_all(&backup_path).unwrap();
        fs::write(backup_path.join("partial.txt"), "partial content").unwrap();

        let context = BackupContext::new();

        // Simulate the entire sequence
        {
            let _guard = context.register_operation(backup_path.clone());

            // Verify operation is tracked and backup exists
            assert!(backup_path.exists());
            let active_ops = context.get_active_operations();
            assert!(active_ops.contains(&backup_path));

            // Simulate Ctrl+C - set interrupt flag
            context.set_interrupted(true);

            // Verify interrupt is detected
            assert!(context.is_interrupted(), "Interrupt flag should be set");

            // Guard drops here when backup function exits due to interrupt
            // With the fix, this should NOT remove the operation from tracking
        }

        // After guard drops, operation should still be tracked (fix)
        let active_ops = context.get_active_operations();
        assert!(
            active_ops.contains(&backup_path),
            "Operation should still be tracked after interrupted guard drops"
        );

        // Backup should still exist (not cleaned up by guard)
        assert!(backup_path.exists());

        // Now signal handler attempts cleanup
        context.cleanup_active_operations_with_mode(true);

        // Verify cleanup worked
        assert!(
            !backup_path.exists(),
            "Backup should be cleaned up by signal handler"
        );

        // Verify tracking is cleared
        let remaining_ops = context.get_active_operations();
        assert!(
            remaining_ops.is_empty(),
            "No operations should remain tracked"
        );
    }
}
