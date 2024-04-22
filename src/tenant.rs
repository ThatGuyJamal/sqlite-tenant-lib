use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, RwLock};

use rusqlite::{Connection, Statement, ToSql};

use crate::{DynamicStdError, SQLResult};

type TenantId = String;

// https://www.sqlite.org/datatype3.html
const STATEMENT_CREATE_MASTER_DB: &str = "
    CREATE TABLE IF NOT EXISTS tenants (
        id TEXT PRIMARY KEY,
        tenant_id TEXT,
        tenant_path
        tenant_has_path INTEGER
        created_at TEXT
    )";

const STATEMENT_SELECT_TENANTS_ON_LOAD: &str = "SELECT id, tenant_path, tenant_has_path FROM tenants";

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
    pub fn new() -> Self
    {
        let master_db = match Connection::open("master.sqlite") {
            Ok(conn) => {
                Self::init_master_db(&conn).expect("Failed to init the master.sqlite db");
                conn
            }
            Err(e) => panic!("{}", e),
        };

        let tenants = match Self::load_tenants(&master_db) {
            Ok(tenants) => tenants,
            Err(e) => panic!("Failed to load tenants: {}", e),
        };

        Self {
            master_db,
            tenants: Arc::new(RwLock::new(tenants)),
        }
    }

    /// Creates the master database if none exist yet.
    fn init_master_db(conn: &Connection) -> SQLResult<()>
    {
        conn.execute(STATEMENT_CREATE_MASTER_DB, [])?;

        Ok(())
    }

    /// Loads the current database tenants into memory. This way developers can get there handles.
    /// todo - when we make the library config, this should be able to be disabled.
    fn load_tenants(master_db: &Connection) -> SQLResult<HashMap<TenantId, TenantConnection>>
    {
        let mut statement: Statement = master_db.prepare(STATEMENT_SELECT_TENANTS_ON_LOAD)?;
        
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

        Ok(())
    }

    /// Removes a tenant connection from the manager
    pub fn remove_tenant(&self, tenant_id: &str) -> SQLResult<(), DynamicStdError>
    {
        let mut tenants = self.tenants.write().unwrap();
        if let Some(_) = tenants.remove(tenant_id) {
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
    pub fn tenant_size(self) -> usize
    {
        self.tenants.read().unwrap().len()
    }
}
