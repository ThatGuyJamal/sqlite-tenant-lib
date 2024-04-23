use std::path::PathBuf;

use flexi_logger::Duplicate;

#[derive(Clone)]
pub enum LogLevel
{
    Info,
    Warn,
    Error,
    Trace,
    Debug,
    All,
}

impl LogLevel
{
    pub fn as_str(&self) -> &'static str
    {
        match self {
            LogLevel::Info => "info",
            LogLevel::Warn => "warn",
            LogLevel::Error => "error",
            LogLevel::Trace => "trace",
            LogLevel::Debug => "debug",
            LogLevel::All => "all",
        }
    }

    pub fn as_dup(&self) -> Duplicate
    {
        match self {
            LogLevel::Info => Duplicate::Info,
            LogLevel::Warn => Duplicate::Warn,
            LogLevel::Error => Duplicate::Error,
            LogLevel::Trace => Duplicate::Trace,
            LogLevel::Debug => Duplicate::Debug,
            LogLevel::All => Duplicate::All,
        }
    }
}

impl Default for LogLevel
{
    fn default() -> Self
    {
        LogLevel::Info
    }
}

/// The config for the tenant manager.
#[derive(Clone)]
pub struct Configuration
{
    /// The path to the sqlite master database that controls the library storage
    pub master_db_path: Option<PathBuf>,
    /// The log level used in the program. If `None` is passed, logging is disabled.
    pub log_level: Option<LogLevel>,
}
