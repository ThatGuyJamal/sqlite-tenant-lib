///! Hello world!
pub mod prelude;
mod statements;
pub mod tenant;

pub use tenant::*;

pub type SQLError = rusqlite::Error;
pub type DynamicStdError = Box<dyn std::error::Error>;
pub type SQLResult<T, E = SQLError> = Result<T, E>;
