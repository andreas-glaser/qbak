pub mod backup;
pub mod config;
pub mod error;
pub mod naming;
pub mod progress;
pub mod utils;

pub use backup::{
    backup_directory, backup_directory_with_progress, backup_file, count_files_and_size,
    count_files_and_size_with_progress, BackupResult,
};
pub use config::{default_config, dump_config, load_config, Config};
pub use error::QbakError;
pub use naming::{generate_backup_name, resolve_collision};
pub use progress::{create_progress_bar, should_show_progress, BackupProgress, ProgressConfig};
pub use utils::{calculate_size, check_available_space, validate_backup_filename, validate_source};

/// Main library result type
pub type Result<T> = std::result::Result<T, QbakError>;
