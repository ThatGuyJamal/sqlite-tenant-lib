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
    // All,
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
            // LogLevel::All => "all",
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
            // LogLevel::All => Duplicate::All,
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
    /// The directory logs will be written to, if `None` it will default to 'logs' in your project root.
    pub log_dir: Option<PathBuf>,
}

#[cfg(test)]
mod tests
{
    use flexi_logger::Logger;
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn test_logger_configuration()
    {
        // Create a temporary directory for logs
        let temp_dir = tempdir().expect("Failed to create temporary directory");

        // Set up test configuration
        let config = Configuration {
            master_db_path: Some(PathBuf::from("test_db.sqlite")),
            log_level: Some(LogLevel::Debug), // Set log level to debug for testing
            log_dir: Some(temp_dir.path().join("logs")),
        };

        // Create a new logger based on the test configuration
        let logger = if let Some(log_level) = config.log_level {
            Some(
                Logger::try_with_str(log_level.as_str())
                    .unwrap()
                    .log_to_file(flexi_logger::FileSpec::default().directory(config.log_dir.unwrap()))
                    .duplicate_to_stdout(log_level.as_dup())
                    .start()
                    .unwrap(),
            )
        } else {
            None
        };

        // Assert that the logger is correctly created
        assert!(logger.is_some());

        // Clean up: Remove any generated log files
        // if let Some(logger) = logger {
        //     logger.shutdown();
        // }
    }
}
