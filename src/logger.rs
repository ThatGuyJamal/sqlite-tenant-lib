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
