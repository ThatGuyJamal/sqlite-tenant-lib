use std::path::PathBuf;

use crate::logger::LogLevel;

/// The config for the tenant manager.
#[derive(Clone)]
pub struct Configuration
{
    /// The path to the sqlite master database that controls the library storage
    pub master_db_path: Option<PathBuf>,
    /// The log level used in the program. If `None` is passed, logging is disabled.
    pub log_level: Option<LogLevel>,
    /// The directory logs will be written to, if `None` it will default to 'logs' in your project root.
    pub log_dir: Option<PathBuf>,
}
