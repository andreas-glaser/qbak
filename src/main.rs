use clap::{Arg, ArgAction, Command};
use qbak::{backup_file, dump_config, load_config, QbakError};
use std::path::Path;
use std::process;

fn main() {
    let result = run();
    match result {
        Ok(exit_code) => process::exit(exit_code),
        Err(error) => {
            eprintln!("Error: {error}");

            // Show suggestions if available
            let suggestions = error.suggestions();
            if !suggestions.is_empty() {
                eprintln!("\nSuggestions:");
                for suggestion in suggestions {
                    eprintln!("  - {suggestion}");
                }
            }

            process::exit(error.exit_code());
        }
    }
}

fn run() -> Result<i32, QbakError> {
    let matches = Command::new("qbak")
        .version("1.4.0-dev")
        .author("Andreas Glaser <andreas.glaser@pm.me>")
        .about("A single-command backup helper for Linux and POSIX systems")
        .long_about(
            "qbak creates timestamped backup copies of files and directories.\n\
             Example: qbak example.txt → example-20250603T145231-qbak.txt",
        )
        .arg(
            Arg::new("targets")
                .help("Files or directories to back up")
                .required(false)
                .num_args(1..)
                .value_name("TARGET"),
        )
        .arg(
            Arg::new("dry-run")
                .short('n')
                .long("dry-run")
                .help("Show what would be backed up without doing it")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .help("Show detailed progress information")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("quiet")
                .short('q')
                .long("quiet")
                .help("Suppress all output except errors")
                .action(ArgAction::SetTrue)
                .conflicts_with_all(["verbose", "progress"]),
        )
        .arg(
            Arg::new("progress")
                .long("progress")
                .help("Force progress indication even for small operations")
                .action(ArgAction::SetTrue)
                .conflicts_with("no-progress"),
        )
        .arg(
            Arg::new("no-progress")
                .long("no-progress")
                .help("Disable progress indication completely")
                .action(ArgAction::SetTrue)
                .conflicts_with("progress"),
        )
        .arg(
            Arg::new("dump-config")
                .long("dump-config")
                .help("Display current configuration settings and exit")
                .action(ArgAction::SetTrue),
        )
        .get_matches();

    // Parse command line flags
    let dump_config_flag = matches.get_flag("dump-config");
    let dry_run = matches.get_flag("dry-run");
    let verbose = matches.get_flag("verbose");
    let quiet = matches.get_flag("quiet");
    let force_progress = matches.get_flag("progress");
    let no_progress = matches.get_flag("no-progress");

    // Load configuration
    let mut config = load_config()
        .map_err(|e| {
            if verbose {
                eprintln!("Warning: Could not load config, using defaults: {e}");
            }
            e
        })
        .unwrap_or_else(|_| qbak::default_config());

    // Apply command line progress flags (they override config)
    if quiet || no_progress {
        config.progress.enabled = false;
    } else if force_progress {
        config.progress.force_enabled = true;
    }

    // Handle dump-config flag early
    if dump_config_flag {
        dump_config(&config)?;
        return Ok(0);
    }

    // Parse targets (only needed if not dumping config)
    let targets: Vec<&str> = if let Some(target_values) = matches.get_many::<String>("targets") {
        target_values.map(|s| s.as_str()).collect()
    } else {
        return Err(QbakError::validation(
            "No targets specified. Use --help for usage information.",
        ));
    };

    // Set up signal handling for graceful cleanup
    setup_signal_handlers();

    let mut success_count = 0;
    let mut error_count = 0;

    // Process each target
    for target_str in targets {
        let target_path = Path::new(target_str);

        match process_target(
            target_path,
            &config,
            dry_run,
            verbose,
            quiet,
            force_progress,
        ) {
            Ok(_) => success_count += 1,
            Err(e) => {
                error_count += 1;

                if e.is_recoverable() {
                    // For recoverable errors, show error but continue
                    if !quiet {
                        eprintln!("Error processing {target_str}: {e}");

                        let suggestions = e.suggestions();
                        if !suggestions.is_empty() && verbose {
                            eprintln!("Suggestions:");
                            for suggestion in suggestions {
                                eprintln!("  - {suggestion}");
                            }
                        }
                    }
                } else {
                    // For non-recoverable errors, fail immediately
                    return Err(e);
                }
            }
        }
    }

    // Summary
    if !quiet && (success_count > 1 || error_count > 0) {
        println!("Backup summary: {success_count} succeeded, {error_count} failed");
    }

    // Return appropriate exit code
    if error_count > 0 {
        Ok(1) // Any failures
    } else {
        Ok(0) // All succeeded
    }
}

fn process_target(
    target: &Path,
    config: &qbak::Config,
    dry_run: bool,
    verbose: bool,
    quiet: bool,
    force_progress: bool,
) -> Result<(), QbakError> {
    if dry_run {
        // Dry run mode - just show what would be done
        let backup_path = qbak::generate_backup_name(target, config)?;
        let final_path = qbak::resolve_collision(&backup_path)?;

        if target.is_dir() {
            // For directories, potentially show scanning progress in dry run
            let should_show_progress =
                config.progress.should_show_progress(0, 0, force_progress) && !quiet;
            let (file_count, total_size) = if should_show_progress {
                qbak::count_files_and_size_with_progress(target, config)?
            } else {
                qbak::count_files_and_size(target, config)?
            };
            let size_str = qbak::utils::format_size(total_size);
            println!(
                "Would create backup: {} ({} files, {size_str})",
                final_path.display(),
                file_count
            );
        } else {
            let size = qbak::calculate_size(target)?;
            let size_str = qbak::utils::format_size(size);
            println!("Would create backup: {} ({size_str})", final_path.display());
        }
        return Ok(());
    }

    // Perform the actual backup
    let result = if target.is_dir() {
        qbak::backup_directory_with_progress(target, config, force_progress || verbose, quiet)?
    } else {
        backup_file(target, config)?
    };

    // Output results based on verbosity
    if verbose {
        println!("Processed: {}", target.display());
        println!("  → {}", result.backup_path.display());
        let files = result.files_processed;
        let size_str = qbak::utils::format_size(result.total_size);
        let duration = result.duration.as_secs_f64();
        println!("  Files: {files}");
        println!("  Size: {size_str}");
        println!("  Duration: {duration:.2}s");
    } else if !quiet {
        let summary = result.summary();
        println!("{summary}");
    }

    Ok(())
}

fn setup_signal_handlers() {
    // Set up signal handlers for graceful cleanup
    #[cfg(unix)]
    {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;

        let interrupted = Arc::new(AtomicBool::new(false));
        let interrupted_clone = interrupted.clone();

        ctrlc::set_handler(move || {
            interrupted_clone.store(true, Ordering::SeqCst);
            eprintln!("\nInterrupted by user. Cleaning up...");

            // Try to clean up any temporary files
            if let Ok(current_dir) = std::env::current_dir() {
                let _ = qbak::backup::cleanup_temp_files(&current_dir);
            }

            process::exit(130);
        })
        .expect("Error setting Ctrl-C handler");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_process_target_file() {
        let dir = tempdir().unwrap();
        let source_path = dir.path().join("test.txt");
        File::create(&source_path).unwrap();

        let config = qbak::default_config();
        let result = process_target(&source_path, &config, false, false, true, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_process_target_dry_run() {
        let dir = tempdir().unwrap();
        let source_path = dir.path().join("test.txt");
        File::create(&source_path).unwrap();

        let config = qbak::default_config();
        let result = process_target(&source_path, &config, true, false, false, false);
        assert!(result.is_ok());

        // In dry run mode, no backup should be created
        let backup_path = qbak::generate_backup_name(&source_path, &config).unwrap();
        assert!(!backup_path.exists());
    }

    #[test]
    fn test_process_target_nonexistent() {
        let dir = tempdir().unwrap();
        let source_path = dir.path().join("nonexistent.txt");

        let config = qbak::default_config();
        let result = process_target(&source_path, &config, false, false, true, false);

        assert!(result.is_err());
        match result.unwrap_err() {
            QbakError::SourceNotFound { .. } => (),
            _ => panic!("Expected SourceNotFound error"),
        }
    }

    #[test]
    fn test_process_target_directory() {
        let dir = tempdir().unwrap();
        let source_dir = dir.path().join("test_dir");
        std::fs::create_dir_all(&source_dir).unwrap();

        // Add a file to the directory
        std::fs::write(source_dir.join("file.txt"), "content").unwrap();

        let config = qbak::default_config();
        let result = process_target(&source_dir, &config, false, false, true, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_process_target_verbose_mode() {
        let dir = tempdir().unwrap();
        let source_path = dir.path().join("test.txt");

        let mut file = File::create(&source_path).unwrap();
        writeln!(file, "Test content").unwrap();

        let config = qbak::default_config();
        // Test verbose mode (should not panic or error)
        let result = process_target(&source_path, &config, false, true, false, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_process_target_dry_run_directory() {
        let dir = tempdir().unwrap();
        let source_dir = dir.path().join("test_dir");
        std::fs::create_dir_all(&source_dir).unwrap();
        std::fs::write(source_dir.join("file.txt"), "content").unwrap();

        let config = qbak::default_config();
        let result = process_target(&source_dir, &config, true, false, false, false);
        assert!(result.is_ok());

        // Verify no backup was actually created
        let backup_path = qbak::generate_backup_name(&source_dir, &config).unwrap();
        assert!(!backup_path.exists());
    }

    #[test]
    fn test_process_target_quiet_mode() {
        let dir = tempdir().unwrap();
        let source_path = dir.path().join("test.txt");
        File::create(&source_path).unwrap();

        let config = qbak::default_config();
        // Test quiet mode
        let result = process_target(&source_path, &config, false, false, true, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_process_target_with_different_config() {
        let dir = tempdir().unwrap();
        let source_path = dir.path().join("test.txt");
        std::fs::write(&source_path, "test content").unwrap();

        let mut config = qbak::default_config();
        config.backup_suffix = "custom".to_string();
        config.preserve_permissions = false;

        let result = process_target(&source_path, &config, false, false, true, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_process_target_large_file() {
        let dir = tempdir().unwrap();
        let source_path = dir.path().join("large.txt");

        // Create a larger file
        let content = "x".repeat(50000);
        std::fs::write(&source_path, content).unwrap();

        let config = qbak::default_config();
        let result = process_target(&source_path, &config, false, true, false, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_process_target_empty_file() {
        let dir = tempdir().unwrap();
        let source_path = dir.path().join("empty.txt");
        File::create(&source_path).unwrap(); // Creates empty file

        let config = qbak::default_config();
        let result = process_target(&source_path, &config, false, false, false, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_process_target_special_characters_in_path() {
        let dir = tempdir().unwrap();
        let source_path = dir.path().join("file with spaces.txt");
        std::fs::write(&source_path, "content").unwrap();

        let config = qbak::default_config();
        let result = process_target(&source_path, &config, false, false, true, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_process_target_unicode_filename() {
        let dir = tempdir().unwrap();
        let source_path = dir.path().join("тест.txt"); // Cyrillic filename
        std::fs::write(&source_path, "unicode content").unwrap();

        let config = qbak::default_config();
        let result = process_target(&source_path, &config, false, false, true, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_process_target_no_extension() {
        let dir = tempdir().unwrap();
        let source_path = dir.path().join("README");
        std::fs::write(&source_path, "readme content").unwrap();

        let config = qbak::default_config();
        let result = process_target(&source_path, &config, false, false, true, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_process_target_multiple_extensions() {
        let dir = tempdir().unwrap();
        let source_path = dir.path().join("archive.tar.gz");
        std::fs::write(&source_path, "archive content").unwrap();

        let config = qbak::default_config();
        let result = process_target(&source_path, &config, false, false, true, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_process_target_hidden_file() {
        let dir = tempdir().unwrap();
        let source_path = dir.path().join(".hidden");
        std::fs::write(&source_path, "hidden content").unwrap();

        let config = qbak::default_config();
        let result = process_target(&source_path, &config, false, false, true, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_process_target_dry_run_verbose() {
        let dir = tempdir().unwrap();
        let source_path = dir.path().join("test.txt");
        std::fs::write(&source_path, "content").unwrap();

        let config = qbak::default_config();
        // Test dry run with verbose output
        let result = process_target(&source_path, &config, true, true, false, false);
        assert!(result.is_ok());
    }
}
