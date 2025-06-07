pub mod backup;
pub mod config;
pub mod error;
pub mod naming;
pub mod utils;

pub use backup::{backup_directory, backup_file, BackupResult};
pub use config::{default_config, dump_config, load_config, Config};
pub use error::QbakError;
pub use naming::{generate_backup_name, resolve_collision};
pub use utils::{calculate_size, check_available_space, validate_backup_filename, validate_source};

/// Main library result type
pub type Result<T> = std::result::Result<T, QbakError>;
