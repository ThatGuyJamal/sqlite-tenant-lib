use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, RwLock};

use rusqlite::{params, Connection, Statement, ToSql};

use crate::statements::SqlStatement;
use crate::{DynamicStdError, SQLResult};

type TenantId = String;

#[allow(dead_code)]
#[derive(Debug)]
/// Rust type representation of our SQL master table.
pub(crate) struct MasterDbTable
{
    id: String,
    tenant_id: String,
    tenant_path: String,
    // 0 = false, 1 = true
    tenant_has_path: i64,
    created_at: String,
}

#[derive(Clone)]
pub struct TenantConnection
{
    #[allow(dead_code)]
    // Connection to the sqlite API.
    // We use a lifetime pointer here so our program can manage connections not control them explicitly.
    pub(crate) connection: Arc<Connection>,
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

pub struct MultiTenantManager
{
    /// The master database manages all the data for other tenants such as lookups, permissions, etc.
    pub(crate) master_db: Connection,
    pub(crate) tenants: Arc<RwLock<HashMap<TenantId, TenantConnection>>>,
}

impl MultiTenantManager
{
    /// Created a new tenant manager.
    ///
    /// A path can be used to create a custom name for the master database. By default, master,sqlite is used.
    pub fn new(path: Option<&str>) -> MultiTenantManager
    {
        let db_path = path.unwrap_or("master.sqlite");
        let master_db = match Connection::open(db_path) {
            Ok(conn) => {
                MultiTenantManager::init_master_db(&conn).expect("Failed to init the master.sqlite db");
                conn
            }
            Err(e) => panic!("{}", e),
        };

        let tenants = match MultiTenantManager::load_tenants(&master_db) {
            Ok(tenants) => tenants,
            Err(e) => panic!("Failed to load tenants: {}", e),
        };

        MultiTenantManager {
            master_db,
            tenants: Arc::new(RwLock::new(tenants)),
        }
    }

    /// Creates the master database if none exist yet.
    fn init_master_db(conn: &Connection) -> SQLResult<()>
    {
        conn.execute(SqlStatement::CreateMasterDb.as_str(), [])?;

        Ok(())
    }

    /// Loads the current database tenants into memory. This way developers can get there handles.
    /// todo - when we make the library config, this should be able to be disabled.
    fn load_tenants(master_db: &Connection) -> SQLResult<HashMap<TenantId, TenantConnection>>
    {
        let mut statement: Statement = master_db.prepare(SqlStatement::SelectTenantsOnLoad.as_str())?;

        // see - https://docs.rs/rusqlite/0.31.0/rusqlite/trait.Params.html#dynamic-parameter-list
        let rows = statement.query_map::<_, &[&dyn ToSql], _>(&[], |row| {
            let id: String = row.get(0)?;
            let path: Option<String> = row.get(1)?;
            let has_path: bool = row.get(2)?;

            let connection: TenantConnection = if has_path {
                TenantConnection::open(Some(path.expect("Expected path, but found None")))?
            } else {
                TenantConnection::open(None::<&Path>)?
            };

            Ok((id, connection))
        })?;

        let mut tenants = HashMap::<TenantId, TenantConnection>::new();
        for result in rows {
            let (id, connection) = result?;
            tenants.insert(id, connection);
        }

        Ok(tenants)
    }

    /// Adds a new tenant to the manager
    ///
    /// `tenant_id` - used to track a connection to a sqlite db. ID generation should be handled by the library user.
    ///
    /// `path` - to the db file. If `None` is passed, the tenant will be created as an in-memory database.
    pub fn add_tenant(&self, tenant_id: &str, path: Option<&Path>) -> SQLResult<(), DynamicStdError>
    {
        let mut tenants = self.tenants.write().unwrap();

        if tenants.contains_key(tenant_id) {
            return Err(DynamicStdError::from(format!("Tenant '{}' already exists", tenant_id)));
        }

        let connection = TenantConnection::open(path)?;

        tenants.insert(tenant_id.to_string(), connection);

        self.master_db.execute(
            "INSERT INTO tenants (tenant_id, tenant_path, tenant_has_path, created_at) VALUES (?1, ?2, ?3, \
             CURRENT_TIMESTAMP)",
            params![
                tenant_id,
                path.as_ref().and_then(|p| p.to_str()).map(|p| p.to_string()),
                path.is_some()
            ],
        )?;

        Ok(())
    }

    /// Removes a tenant connection from the manager
    pub fn remove_tenant(&self, tenant_id: &str) -> SQLResult<(), DynamicStdError>
    {
        let mut tenants = self.tenants.write().unwrap();
        if let Some(_) = tenants.remove(tenant_id) {
            self.master_db
                .execute("DELETE FROM tenants WHERE id = ?1", params![tenant_id])?;
            Ok(())
        } else {
            Err(DynamicStdError::from(format!("Tenant '{}' not found", tenant_id)))
        }
    }

    /// Get a tenant connection based on id
    pub fn get_connection(&self, tenant_id: &str) -> SQLResult<Option<TenantConnection>, DynamicStdError>
    {
        let tenants = self.tenants.read().unwrap();

        if let Some(connection) = tenants.get(tenant_id).cloned() {
            Ok(Option::from(connection))
        } else {
            Ok(None)
        }
    }

    /// Gets the current amount of tenants in the database.
    pub fn tenant_size(&self) -> usize
    {
        self.tenants.read().unwrap().len()
    }
}

#[cfg(test)]
mod tests
{
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn test_master_db_setup()
    {
        let temp_dir = tempdir().expect("Failed to create temporary directory");
        let db_path = temp_dir.path().join("master_test_1.sqlite");
        let _ = MultiTenantManager::new(Some(db_path.to_str().unwrap()));
        assert!(db_path.exists(), "master.sqlite file does not exist");
    }

    #[test]
    fn test_add_and_remove_tenants()
    {
        let temp_dir = tempdir().expect("Failed to create temporary directory");
        let db_path = temp_dir.path().join("master_test_2.sqlite");
        let manager = MultiTenantManager::new(Some(db_path.to_str().unwrap()));

        // Add 3 tenants
        manager.add_tenant("tenant1", None).expect("Failed to add tenant1");
        manager.add_tenant("tenant2", None).expect("Failed to add tenant2");
        manager.add_tenant("tenant3", None).expect("Failed to add tenant3");

        // Check if the size of tenants hashmap is 3
        assert_eq!(manager.tenant_size(), 3);

        // Remove the tenants
        manager.remove_tenant("tenant1").expect("Failed to remove tenant1");
        manager.remove_tenant("tenant2").expect("Failed to remove tenant2");
        manager.remove_tenant("tenant3").expect("Failed to remove tenant3");

        // Check if the size of tenants hashmap is 0 after removal
        assert_eq!(manager.tenant_size(), 0);
    }
}