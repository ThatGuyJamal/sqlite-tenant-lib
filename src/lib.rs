use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, RwLock};

use rusqlite::{Connection, Error as SQLError};

mod test;

type StdError = Box<dyn std::error::Error>;
type SQLResult<T, E = SQLError> = Result<T, E>;
type TenantId = String;

#[derive(Clone)]
pub(crate) struct TenantConnection
{
    // Connection to the sqlite API.
    // We use a lifetime pointer here so our program can manage connections not control them explicitly.
    connection: Arc<Connection>,
}

impl TenantConnection
{
    /// Opens a connection to the sqlite database
    ///
    /// If no path is provided, then the library defaults to in memory sqlite only.
    pub(crate) fn open<P: AsRef<Path>>(path: Option<P>) -> SQLResult<Self>
    {
        if let Some(p) = path {
            Ok(Self {
                connection: Arc::new(Connection::open(p)?)
            })
        } else {
            Ok(Self {
                connection: Arc::new(Connection::open_in_memory()?,)
            })
        }
    }
}

pub(crate) struct MultiTenantManager
{
    tenants: Arc<RwLock<HashMap<TenantId, TenantConnection>>>
}

impl MultiTenantManager
{
    pub(crate) fn new() -> Self
    {
        Self {
            tenants: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Adds a new tenant to the manager
    ///
    /// `tenant_id` - used to track a connection to a sqlite db. ID generation should be handled by the library user.
    ///
    /// `path` - to the db file. If `None` is passed, the tenant will be created as an in-memory database.
    pub fn add_tenant(&self, tenant_id: &str, path: Option<&Path>) -> SQLResult<(), StdError>
    {
        let mut tenants = self.tenants.write().unwrap();

        if tenants.contains_key(tenant_id) {
            return Err(StdError::from(format!("Tenant '{}' already exists", tenant_id)));
        }

        let connection = TenantConnection::open(path)?;

        tenants.insert(tenant_id.to_string(), connection);

        Ok(())
    }

    /// Removes a tenant connection from the manager
    pub fn remove_tenant(&self, tenant_id: &str) -> SQLResult<(), StdError>
    {
        let mut tenants = self.tenants.write().unwrap();
        if let Some(_) = tenants.remove(tenant_id) {
            Ok(())
        } else {
            Err(StdError::from(format!("Tenant '{}' not found", tenant_id)))
        }
    }

    /// Get a tenant connection based on id
    pub fn get_connection(&self, tenant_id: &str) -> SQLResult<Option<TenantConnection>, StdError>
    {
        let tenants = self.tenants.read().unwrap();

        if let Some(connection) = tenants.get(tenant_id).cloned() {
            Ok(Option::from(connection))
        } else {
            Ok(None)
        }
    }
}
