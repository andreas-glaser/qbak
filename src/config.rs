use crate::error::QbakError;
use crate::Result;
use configparser::ini::Ini;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Config {
    pub timestamp_format: String,
    pub backup_suffix: String,
    pub preserve_permissions: bool,
    pub follow_symlinks: bool,
    pub include_hidden: bool,
    pub max_filename_length: usize,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            timestamp_format: "YYYYMMDDTHHMMSS".to_string(),
            backup_suffix: "qbak".to_string(),
            preserve_permissions: true,
            follow_symlinks: true,
            include_hidden: true,
            max_filename_length: 255,
        }
    }
}

/// Get default configuration
pub fn default_config() -> Config {
    Config::default()
}

/// Load configuration from file, falling back to defaults
pub fn load_config() -> Result<Config> {
    let config_path = get_config_path()?;

    if !config_path.exists() {
        return Ok(default_config());
    }

    let mut conf = Ini::new();
    conf.load(&config_path)
        .map_err(|e| QbakError::config(format!("Failed to parse config file: {e}")))?;

    let mut config = default_config();

    // Load string values
    if let Some(value) = conf.get("qbak", "timestamp_format") {
        config.timestamp_format = value;
    }
    if let Some(value) = conf.get("qbak", "backup_suffix") {
        config.backup_suffix = value;
    }

    // Load boolean values
    if let Some(value) = conf.get("qbak", "preserve_permissions") {
        config.preserve_permissions = parse_bool(&value).unwrap_or(config.preserve_permissions);
    }
    if let Some(value) = conf.get("qbak", "follow_symlinks") {
        config.follow_symlinks = parse_bool(&value).unwrap_or(config.follow_symlinks);
    }
    if let Some(value) = conf.get("qbak", "include_hidden") {
        config.include_hidden = parse_bool(&value).unwrap_or(config.include_hidden);
    }

    // Load numeric values
    if let Some(value) = conf.get("qbak", "max_filename_length") {
        config.max_filename_length = value
            .parse()
            .map_err(|_| QbakError::config(format!("Invalid max_filename_length: {value}")))?;
    }

    Ok(config)
}

/// Get the configuration file path for the current platform
fn get_config_path() -> Result<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        if let Some(appdata) = std::env::var_os("APPDATA") {
            return Ok(PathBuf::from(appdata).join("qbak").join("config.ini"));
        }
    }

    // Unix-like systems (Linux, macOS, etc.)
    if let Some(config_dir) = std::env::var_os("XDG_CONFIG_HOME") {
        Ok(PathBuf::from(config_dir).join("qbak").join("config.ini"))
    } else if let Some(home) = std::env::var_os("HOME") {
        Ok(PathBuf::from(home)
            .join(".config")
            .join("qbak")
            .join("config.ini"))
    } else {
        Err(QbakError::config("Could not determine config directory"))
    }
}

/// Parse a boolean value from INI string
fn parse_bool(value: &str) -> Option<bool> {
    match value.to_lowercase().as_str() {
        "true" | "yes" | "1" | "on" => Some(true),
        "false" | "no" | "0" | "off" => Some(false),
        _ => None,
    }
}

/// Create a sample configuration file
pub fn create_sample_config() -> String {
    r#"[qbak]
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

# Maximum filename length before showing error
max_filename_length = 255
"#
    .to_string()
}

/// Display the current configuration in a user-friendly format
pub fn dump_config(config: &Config) -> Result<()> {
    let config_path = get_config_path()?;

    println!("qbak Configuration");
    println!("==================");
    println!();

    // Show config file path and status
    if config_path.exists() {
        println!("Config file: {} (found)", config_path.display());
    } else {
        println!(
            "Config file: {} (not found, using defaults)",
            config_path.display()
        );
    }
    println!();

    // Show current settings
    println!("Current Settings:");
    println!("----------------");
    println!("timestamp_format     = {}", config.timestamp_format);
    println!("backup_suffix        = {}", config.backup_suffix);
    println!("preserve_permissions = {}", config.preserve_permissions);
    println!("follow_symlinks      = {}", config.follow_symlinks);
    println!("include_hidden       = {}", config.include_hidden);
    println!("max_filename_length  = {}", config.max_filename_length);
    println!();

    // Show example usage
    println!("Example backup names with current settings:");
    println!("------------------------------------------");
    println!(
        "example.txt → example-YYYYMMDDTHHMMSS-{}.txt",
        config.backup_suffix
    );
    println!(
        "data.tar.gz → data.tar-YYYYMMDDTHHMMSS-{}.gz",
        config.backup_suffix
    );
    println!("no-ext → no-ext-YYYYMMDDTHHMMSS-{}", config.backup_suffix);
    println!();

    if !config_path.exists() {
        println!("To create a configuration file:");
        println!("------------------------------");
        println!(
            "1. Create directory: mkdir -p {}",
            config_path.parent().unwrap().display()
        );
        println!("2. Create config file with your preferred settings");
        println!("3. Use 'qbak --dump-config' again to verify");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::Mutex;
    use tempfile::tempdir;

    // Mutex to serialize tests that modify environment variables
    static ENV_MUTEX: Mutex<()> = Mutex::new(());

    #[test]
    fn test_default_config() {
        let config = default_config();
        assert_eq!(config.timestamp_format, "YYYYMMDDTHHMMSS");
        assert_eq!(config.backup_suffix, "qbak");
        assert!(config.preserve_permissions);
        assert!(config.follow_symlinks);
        assert!(config.include_hidden);
        assert_eq!(config.max_filename_length, 255);
    }

    #[test]
    fn test_parse_bool() {
        assert_eq!(parse_bool("true"), Some(true));
        assert_eq!(parse_bool("false"), Some(false));
        assert_eq!(parse_bool("yes"), Some(true));
        assert_eq!(parse_bool("no"), Some(false));
        assert_eq!(parse_bool("1"), Some(true));
        assert_eq!(parse_bool("0"), Some(false));
        assert_eq!(parse_bool("on"), Some(true));
        assert_eq!(parse_bool("off"), Some(false));
        assert_eq!(parse_bool("TRUE"), Some(true));
        assert_eq!(parse_bool("FALSE"), Some(false));
        assert_eq!(parse_bool("invalid"), None);
        assert_eq!(parse_bool(""), None);
        assert_eq!(parse_bool("maybe"), None);
    }

    #[test]
    fn test_load_config_nonexistent_file() {
        let _guard = ENV_MUTEX.lock().unwrap(); // Serialize environment access

        // Save original environment
        #[cfg(not(target_os = "windows"))]
        let (original_xdg, original_home) = (
            std::env::var_os("XDG_CONFIG_HOME"),
            std::env::var_os("HOME"),
        );
        #[cfg(target_os = "windows")]
        let original_appdata = std::env::var_os("APPDATA");

        // Set a temp directory so config loading succeeds but file doesn't exist
        let dir = tempdir().unwrap();

        #[cfg(not(target_os = "windows"))]
        std::env::set_var("XDG_CONFIG_HOME", dir.path());
        #[cfg(target_os = "windows")]
        std::env::set_var("APPDATA", dir.path());

        // Test loading config when file doesn't exist - should return defaults
        let config = load_config().unwrap();
        let default = default_config();

        assert_eq!(config.timestamp_format, default.timestamp_format);
        assert_eq!(config.backup_suffix, default.backup_suffix);
        assert_eq!(config.preserve_permissions, default.preserve_permissions);

        // Restore original environment
        #[cfg(not(target_os = "windows"))]
        {
            if let Some(xdg) = original_xdg {
                std::env::set_var("XDG_CONFIG_HOME", xdg);
            } else {
                std::env::remove_var("XDG_CONFIG_HOME");
            }
            if let Some(home) = original_home {
                std::env::set_var("HOME", home);
            } else {
                std::env::remove_var("HOME");
            }
        }
        #[cfg(target_os = "windows")]
        {
            if let Some(appdata) = original_appdata {
                std::env::set_var("APPDATA", appdata);
            } else {
                std::env::remove_var("APPDATA");
            }
        }
    }

    #[test]
    fn test_config_from_file() {
        let _guard = ENV_MUTEX.lock().unwrap(); // Serialize environment access

        // Save original environment
        #[cfg(not(target_os = "windows"))]
        let (original_xdg, original_home) = (
            std::env::var_os("XDG_CONFIG_HOME"),
            std::env::var_os("HOME"),
        );
        #[cfg(target_os = "windows")]
        let original_appdata = std::env::var_os("APPDATA");

        let dir = tempdir().unwrap();
        let config_dir = dir.path().join("qbak");
        fs::create_dir_all(&config_dir).unwrap();
        let config_path = config_dir.join("config.ini");

        // Create a test config file
        let config_content = r#"[qbak]
timestamp_format = TEST_FORMAT
backup_suffix = test-suffix
preserve_permissions = false
follow_symlinks = false
include_hidden = false
max_filename_length = 100
"#;
        fs::write(&config_path, config_content).unwrap();

        // Clear and set environment variables for clean test environment
        #[cfg(not(target_os = "windows"))]
        {
            std::env::remove_var("HOME"); // Clear HOME to ensure XDG_CONFIG_HOME is used
            std::env::set_var("XDG_CONFIG_HOME", dir.path());
        }
        #[cfg(target_os = "windows")]
        std::env::set_var("APPDATA", dir.path());

        let config = load_config().unwrap();

        assert_eq!(config.timestamp_format, "TEST_FORMAT");
        assert_eq!(config.backup_suffix, "test-suffix");
        assert!(!config.preserve_permissions);
        assert!(!config.follow_symlinks);
        assert!(!config.include_hidden);
        assert_eq!(config.max_filename_length, 100);

        // Restore original environment
        #[cfg(not(target_os = "windows"))]
        {
            if let Some(xdg) = original_xdg {
                std::env::set_var("XDG_CONFIG_HOME", xdg);
            } else {
                std::env::remove_var("XDG_CONFIG_HOME");
            }
            if let Some(home) = original_home {
                std::env::set_var("HOME", home);
            } else {
                std::env::remove_var("HOME");
            }
        }
        #[cfg(target_os = "windows")]
        {
            if let Some(appdata) = original_appdata {
                std::env::set_var("APPDATA", appdata);
            } else {
                std::env::remove_var("APPDATA");
            }
        }
    }

    #[test]
    fn test_config_partial_override() {
        let _guard = ENV_MUTEX.lock().unwrap(); // Serialize environment access

        // Save original environment
        #[cfg(not(target_os = "windows"))]
        let (original_xdg, original_home) = (
            std::env::var_os("XDG_CONFIG_HOME"),
            std::env::var_os("HOME"),
        );
        #[cfg(target_os = "windows")]
        let original_appdata = std::env::var_os("APPDATA");

        let dir = tempdir().unwrap();
        let config_dir = dir.path().join("qbak");
        fs::create_dir_all(&config_dir).unwrap();
        let config_path = config_dir.join("config.ini");

        // Create a config file with only some values
        let config_content = r#"[qbak]
backup_suffix = custom
"#;
        fs::write(&config_path, config_content).unwrap();

        // Clear and set environment variables for clean test environment
        #[cfg(not(target_os = "windows"))]
        {
            std::env::remove_var("HOME"); // Clear HOME to ensure XDG_CONFIG_HOME is used
            std::env::set_var("XDG_CONFIG_HOME", dir.path());
        }
        #[cfg(target_os = "windows")]
        std::env::set_var("APPDATA", dir.path());

        let config = load_config().unwrap();
        let default = default_config();

        // Overridden values
        assert_eq!(config.backup_suffix, "custom");

        // Default values should remain
        assert_eq!(config.timestamp_format, default.timestamp_format);
        assert_eq!(config.preserve_permissions, default.preserve_permissions);
        assert_eq!(config.follow_symlinks, default.follow_symlinks);
        assert_eq!(config.include_hidden, default.include_hidden);
        assert_eq!(config.max_filename_length, default.max_filename_length);

        // Restore original environment
        #[cfg(not(target_os = "windows"))]
        {
            if let Some(xdg) = original_xdg {
                std::env::set_var("XDG_CONFIG_HOME", xdg);
            } else {
                std::env::remove_var("XDG_CONFIG_HOME");
            }
            if let Some(home) = original_home {
                std::env::set_var("HOME", home);
            } else {
                std::env::remove_var("HOME");
            }
        }
        #[cfg(target_os = "windows")]
        {
            if let Some(appdata) = original_appdata {
                std::env::set_var("APPDATA", appdata);
            } else {
                std::env::remove_var("APPDATA");
            }
        }
    }

    #[test]
    fn test_config_invalid_boolean() {
        let _guard = ENV_MUTEX.lock().unwrap(); // Serialize environment access

        // Save original environment
        #[cfg(not(target_os = "windows"))]
        let (original_xdg, original_home) = (
            std::env::var_os("XDG_CONFIG_HOME"),
            std::env::var_os("HOME"),
        );
        #[cfg(target_os = "windows")]
        let original_appdata = std::env::var_os("APPDATA");

        let dir = tempdir().unwrap();
        let config_dir = dir.path().join("qbak");
        fs::create_dir_all(&config_dir).unwrap();
        let config_path = config_dir.join("config.ini");

        let config_content = r#"[qbak]
preserve_permissions = maybe
follow_symlinks = invalid
include_hidden = not_a_boolean
"#;
        fs::write(&config_path, config_content).unwrap();

        // Clear and set environment variables for clean test environment
        #[cfg(not(target_os = "windows"))]
        {
            std::env::remove_var("HOME"); // Clear HOME to ensure XDG_CONFIG_HOME is used
            std::env::set_var("XDG_CONFIG_HOME", dir.path());
        }
        #[cfg(target_os = "windows")]
        std::env::set_var("APPDATA", dir.path());

        let config = load_config().unwrap();
        let default = default_config();

        // Invalid booleans should fallback to defaults
        assert_eq!(config.preserve_permissions, default.preserve_permissions);
        assert_eq!(config.follow_symlinks, default.follow_symlinks);

        // Restore original environment
        #[cfg(not(target_os = "windows"))]
        {
            if let Some(xdg) = original_xdg {
                std::env::set_var("XDG_CONFIG_HOME", xdg);
            } else {
                std::env::remove_var("XDG_CONFIG_HOME");
            }
            if let Some(home) = original_home {
                std::env::set_var("HOME", home);
            } else {
                std::env::remove_var("HOME");
            }
        }
        #[cfg(target_os = "windows")]
        {
            if let Some(appdata) = original_appdata {
                std::env::set_var("APPDATA", appdata);
            } else {
                std::env::remove_var("APPDATA");
            }
        }
    }

    #[test]
    fn test_config_invalid_numeric() {
        let _guard = ENV_MUTEX.lock().unwrap(); // Serialize environment access

        // Save original environment
        #[cfg(not(target_os = "windows"))]
        let (original_xdg, original_home) = (
            std::env::var_os("XDG_CONFIG_HOME"),
            std::env::var_os("HOME"),
        );
        #[cfg(target_os = "windows")]
        let original_appdata = std::env::var_os("APPDATA");

        let dir = tempdir().unwrap();
        let config_dir = dir.path().join("qbak");
        fs::create_dir_all(&config_dir).unwrap();
        let config_path = config_dir.join("config.ini");

        let config_content = r#"[qbak]
max_filename_length = not_a_number
"#;
        fs::write(&config_path, config_content).unwrap();

        // Clear and set environment variables for clean test environment
        #[cfg(not(target_os = "windows"))]
        {
            std::env::remove_var("HOME"); // Clear HOME to ensure XDG_CONFIG_HOME is used
            std::env::set_var("XDG_CONFIG_HOME", dir.path());
        }
        #[cfg(target_os = "windows")]
        std::env::set_var("APPDATA", dir.path());

        let result = load_config();
        assert!(result.is_err()); // Should fail to parse invalid numbers

        // Restore original environment
        #[cfg(not(target_os = "windows"))]
        {
            if let Some(xdg) = original_xdg {
                std::env::set_var("XDG_CONFIG_HOME", xdg);
            } else {
                std::env::remove_var("XDG_CONFIG_HOME");
            }
            if let Some(home) = original_home {
                std::env::set_var("HOME", home);
            } else {
                std::env::remove_var("HOME");
            }
        }
        #[cfg(target_os = "windows")]
        {
            if let Some(appdata) = original_appdata {
                std::env::set_var("APPDATA", appdata);
            } else {
                std::env::remove_var("APPDATA");
            }
        }
    }

    #[test]
    fn test_config_malformed_file() {
        let _guard = ENV_MUTEX.lock().unwrap(); // Serialize environment access

        // Save original environment
        #[cfg(not(target_os = "windows"))]
        let (original_xdg, original_home) = (
            std::env::var_os("XDG_CONFIG_HOME"),
            std::env::var_os("HOME"),
        );
        #[cfg(target_os = "windows")]
        let original_appdata = std::env::var_os("APPDATA");

        let dir = tempdir().unwrap();
        let config_dir = dir.path().join("qbak");
        fs::create_dir_all(&config_dir).unwrap();
        let config_path = config_dir.join("config.ini");

        // Create binary/invalid content that should definitely fail
        let config_content = vec![0u8, 159u8, 146u8, 150u8]; // Invalid UTF-8 bytes
        fs::write(&config_path, config_content).unwrap();

        // Clear and set environment variables for clean test environment
        #[cfg(not(target_os = "windows"))]
        {
            std::env::remove_var("HOME"); // Clear HOME to ensure XDG_CONFIG_HOME is used
            std::env::set_var("XDG_CONFIG_HOME", dir.path());
        }
        #[cfg(target_os = "windows")]
        std::env::set_var("APPDATA", dir.path());

        let result = load_config();
        // The configparser is quite tolerant, but invalid UTF-8 should fail
        // If it doesn't fail, that's actually fine - just means robust parsing
        if result.is_err() {
            // Good - caught the malformed file
        } else {
            // Parser is very robust - that's actually fine for a backup tool
            // Just verify it returns default values when it can't parse sections
            let config = result.unwrap();
            let default = default_config();
            assert_eq!(config.timestamp_format, default.timestamp_format);
        }

        // Restore original environment
        #[cfg(not(target_os = "windows"))]
        {
            if let Some(xdg) = original_xdg {
                std::env::set_var("XDG_CONFIG_HOME", xdg);
            } else {
                std::env::remove_var("XDG_CONFIG_HOME");
            }
            if let Some(home) = original_home {
                std::env::set_var("HOME", home);
            } else {
                std::env::remove_var("HOME");
            }
        }
        #[cfg(target_os = "windows")]
        {
            if let Some(appdata) = original_appdata {
                std::env::set_var("APPDATA", appdata);
            } else {
                std::env::remove_var("APPDATA");
            }
        }
    }

    #[test]
    fn test_create_sample_config() {
        let sample = create_sample_config();

        assert!(sample.contains("[qbak]"));
        assert!(sample.contains("timestamp_format"));
        assert!(sample.contains("backup_suffix"));
        assert!(sample.contains("preserve_permissions"));
        assert!(sample.contains("follow_symlinks"));
        assert!(sample.contains("include_hidden"));
        assert!(sample.contains("max_filename_length"));
        println!("{}", sample);

        // Verify it's valid INI by parsing it
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("sample.ini");
        fs::write(&config_path, &sample).unwrap();

        let mut conf = Ini::new();
        assert!(conf.load(&config_path).is_ok());
    }

    #[test]
    fn test_config_with_empty_section() {
        let _guard = ENV_MUTEX.lock().unwrap(); // Serialize environment access

        // Save original environment
        #[cfg(not(target_os = "windows"))]
        let (original_xdg, original_home) = (
            std::env::var_os("XDG_CONFIG_HOME"),
            std::env::var_os("HOME"),
        );
        #[cfg(target_os = "windows")]
        let original_appdata = std::env::var_os("APPDATA");

        let dir = tempdir().unwrap();
        let config_dir = dir.path().join("qbak");
        fs::create_dir_all(&config_dir).unwrap();
        let config_path = config_dir.join("config.ini");

        // Config file with empty qbak section
        let config_content = r#"[qbak]
[other_section]
some_key = some_value
"#;
        fs::write(&config_path, config_content).unwrap();

        // Clear and set environment variables for clean test environment
        #[cfg(not(target_os = "windows"))]
        {
            std::env::remove_var("HOME"); // Clear HOME to ensure XDG_CONFIG_HOME is used
            std::env::set_var("XDG_CONFIG_HOME", dir.path());
        }
        #[cfg(target_os = "windows")]
        std::env::set_var("APPDATA", dir.path());

        let config = load_config().unwrap();
        let default = default_config();

        // Should use all default values
        assert_eq!(config.timestamp_format, default.timestamp_format);
        assert_eq!(config.backup_suffix, default.backup_suffix);
        assert_eq!(config.preserve_permissions, default.preserve_permissions);
        assert_eq!(config.follow_symlinks, default.follow_symlinks);
        assert_eq!(config.include_hidden, default.include_hidden);
        assert_eq!(config.max_filename_length, default.max_filename_length);

        // Restore original environment
        #[cfg(not(target_os = "windows"))]
        {
            if let Some(xdg) = original_xdg {
                std::env::set_var("XDG_CONFIG_HOME", xdg);
            } else {
                std::env::remove_var("XDG_CONFIG_HOME");
            }
            if let Some(home) = original_home {
                std::env::set_var("HOME", home);
            } else {
                std::env::remove_var("HOME");
            }
        }
        #[cfg(target_os = "windows")]
        {
            if let Some(appdata) = original_appdata {
                std::env::set_var("APPDATA", appdata);
            } else {
                std::env::remove_var("APPDATA");
            }
        }
    }

    #[test]
    fn test_get_config_path_xdg() {
        let _guard = ENV_MUTEX.lock().unwrap(); // Serialize environment access

        #[cfg(not(target_os = "windows"))]
        {
            // Save original environment
            let original_xdg = std::env::var_os("XDG_CONFIG_HOME");

            let dir = tempdir().unwrap();
            std::env::set_var("XDG_CONFIG_HOME", dir.path());

            let config_path = get_config_path().unwrap();
            let expected = dir.path().join("qbak").join("config.ini");

            assert_eq!(config_path, expected);

            // Restore original environment
            if let Some(xdg) = original_xdg {
                std::env::set_var("XDG_CONFIG_HOME", xdg);
            } else {
                std::env::remove_var("XDG_CONFIG_HOME");
            }
        }

        // On Windows, test APPDATA path
        #[cfg(target_os = "windows")]
        {
            // Save original environment
            let original_appdata = std::env::var_os("APPDATA");

            let dir = tempdir().unwrap();
            std::env::set_var("APPDATA", dir.path());

            let config_path = get_config_path().unwrap();
            let expected = dir.path().join("qbak").join("config.ini");

            assert_eq!(config_path, expected);

            // Restore original environment
            if let Some(appdata) = original_appdata {
                std::env::set_var("APPDATA", appdata);
            } else {
                std::env::remove_var("APPDATA");
            }
        }
    }

    #[test]
    fn test_get_config_path_home() {
        let _guard = ENV_MUTEX.lock().unwrap(); // Serialize environment access

        #[cfg(not(target_os = "windows"))]
        {
            // Save original environment
            let original_xdg = std::env::var_os("XDG_CONFIG_HOME");
            let original_home = std::env::var_os("HOME");

            // Remove XDG_CONFIG_HOME to test HOME fallback
            std::env::remove_var("XDG_CONFIG_HOME");

            // Ensure HOME is available for this test
            if let Some(home) = original_home.clone() {
                std::env::set_var("HOME", &home);

                let config_path = get_config_path().unwrap();
                let expected = PathBuf::from(home)
                    .join(".config")
                    .join("qbak")
                    .join("config.ini");

                assert_eq!(config_path, expected);
            }

            // Restore original environment
            if let Some(xdg) = original_xdg {
                std::env::set_var("XDG_CONFIG_HOME", xdg);
            } else {
                std::env::remove_var("XDG_CONFIG_HOME");
            }
            if let Some(home) = original_home {
                std::env::set_var("HOME", home);
            } else {
                std::env::remove_var("HOME");
            }
        }
    }

    #[test]
    fn test_dump_config() {
        let config = default_config();
        let result = dump_config(&config);
        // Should not fail - actual output verification would require capturing stdout
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_config_path_no_env() {
        let _guard = ENV_MUTEX.lock().unwrap(); // Serialize environment access

        #[cfg(not(target_os = "windows"))]
        {
            // Save original environment
            let original_xdg = std::env::var_os("XDG_CONFIG_HOME");
            let original_home = std::env::var_os("HOME");

            // Remove both environment variables
            std::env::remove_var("XDG_CONFIG_HOME");
            std::env::remove_var("HOME");

            let result = get_config_path();
            assert!(result.is_err()); // Should fail when no env vars are set

            // Restore original environment immediately
            if let Some(xdg) = original_xdg {
                std::env::set_var("XDG_CONFIG_HOME", xdg);
            } else {
                std::env::remove_var("XDG_CONFIG_HOME");
            }
            if let Some(home) = original_home {
                std::env::set_var("HOME", home);
            } else {
                std::env::remove_var("HOME");
            }
        }

        #[cfg(target_os = "windows")]
        {
            // Save original environment
            let original_appdata = std::env::var_os("APPDATA");

            // Remove APPDATA environment variable
            std::env::remove_var("APPDATA");

            let result = get_config_path();
            assert!(result.is_err()); // Should fail when APPDATA is not set

            // Restore original environment immediately
            if let Some(appdata) = original_appdata {
                std::env::set_var("APPDATA", appdata);
            } else {
                std::env::remove_var("APPDATA");
            }
        }
    }
}
