use std::path::Path;
use std::sync::Arc;

use rusqlite::Connection;

use crate::error::SQLResult;

#[derive(Clone)]
pub struct TenantConnection
{
    // Connection to the sqlite API.
    pub connection: Arc<Connection>,
}

impl TenantConnection
{
    /// Opens a connection to the sqlite database
    ///
    /// If `None` is provided, then the library defaults to in memory sqlite only.
    pub fn open<P: AsRef<Path>>(path: Option<P>) -> SQLResult<Self>
    {
        if let Some(p) = path {
            Ok(Self {
                connection: Arc::new(Connection::open(p)?),
            })
        } else {
            Ok(Self {
                connection: Arc::new(Connection::open_in_memory()?),
            })
        }
    }
}
