use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use flexi_logger::{FileSpec, Logger, LoggerHandle};
use log::{debug, error, info, warn};
use rusqlite::{params, Connection, Statement, ToSql};

use crate::statements::SqlStatement;
use crate::{Configuration, DynamicStdError, MultiTenantError, SQLResult};

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
    #[allow(dead_code)]
    pub(super) logger: Option<LoggerHandle>,
    #[allow(dead_code)]
    pub(super) config: Configuration,
}

impl MultiTenantManager
{
    /// Created a new tenant manager.
    pub fn new(config: Configuration) -> SQLResult<Self>
    {
        let mut master_db = match config.master_db_path.clone() {
            Some(path) => Connection::open(path),
            None => Connection::open_in_memory(),
        }
        .unwrap_or_else(|e| panic!("Failed to open database: {}", e));

        // todo - handle this error conversion with '?'
        MultiTenantManager::init_master_db(&mut master_db).expect("Failed to init master database");

        let tenants = match MultiTenantManager::load_tenants(&master_db) {
            Ok(tenants) => tenants,
            Err(e) => panic!("Failed to load tenants: {}", e),
        };

        let logger = if let Some(log_level) = config.log_level.clone() {
            // If log_level is Some, initialize the logger
            match Logger::try_with_str(log_level.as_str()) {
                Ok(logger_builder) => {
                    match logger_builder
                        .log_to_file(FileSpec::default().directory(config.log_dir.clone().unwrap_or(PathBuf::from("logs"))))
                        .duplicate_to_stdout(log_level.as_dup())
                        .format(flexi_logger::detailed_format)
                        .start()
                    {
                        Ok(logger_handle) => Some(logger_handle),
                        Err(err) => {
                            eprintln!("Error starting logger: {}", err);
                            None
                        }
                    }
                }
                Err(err) => {
                    eprintln!("Error creating logger: {}", err);
                    None
                }
            }
        } else {
            None
        };

        info!("MultiTenantManager Initialized");

        Ok(Self {
            master_db,
            tenants: Arc::new(RwLock::new(tenants)),
            logger,
            config: config.clone(),
        })
    }

    /// Adds a new tenant to the manager
    ///
    /// `tenant_id` - used to track a connection to a sqlite db. ID generation should be handled by the library user.
    ///
    /// `path` - to the db file. If `None` is passed, the tenant will be created as an in-memory database.
    pub fn add_tenant(&mut self, tenant_id: &str, path: Option<PathBuf>) -> SQLResult<(), MultiTenantError>
    {
        let mut tenants = self.tenants.write().unwrap();

        if tenants.contains_key(tenant_id) {
            debug!("Attempted to add tenant ({}) but it already exist.", tenant_id);
            return Err(MultiTenantError::TenantAlreadyExists(tenant_id.to_string()));
        }

        let connection = TenantConnection::open(path.clone())?;

        // Begin a transaction
        let tx = self.master_db.transaction()?;

        tx.execute(
            SqlStatement::InsertAddTenant.as_str(),
            params![
            tenant_id,
            path.as_ref().and_then(|p| p.to_str()).unwrap_or_default(), // Default to empty string if path is None
            path.is_some()
        ],
        )?;

        if let Err(err) = tx.commit() {
            debug!("Failed to commit transaction: {}", err);
            return Err(MultiTenantError::DatabaseError(format!("Failed to commit transaction: {}", err)));
        }

        tenants.insert(tenant_id.to_string(), connection);

        info!("Added ({}) tenant.", tenant_id);

        Ok(())
    }

    /// Removes a tenant connection from the manager
    pub fn remove_tenant(&mut self, tenant_id: &str) -> SQLResult<(), MultiTenantError>
    {
        let mut tenants = self.tenants.write().unwrap();

        if let Some(_) = tenants.remove(tenant_id) {
            // Begin a transaction
            let tx = self.master_db.transaction()?;

            tx.execute(SqlStatement::DeleteRemoveTenant.as_str(), params![tenant_id])?;

            if let Err(err) = tx.commit() {
                debug!("Failed to commit transaction: {}", err);
                return Err(MultiTenantError::DatabaseError(format!("Failed to commit transaction: {}", err)));
            }

            debug!("Deleted ({}) tenant.", tenant_id);
            Ok(())
        } else {
            error!("Attempted to delete tenant ({}) that does not exist.", tenant_id);
            return Err(MultiTenantError::TenantNotFound(tenant_id.to_string()));
        }
    }


    /// Get a tenant connection based on id
    pub fn get_connection(&self, tenant_id: &str) -> SQLResult<Option<TenantConnection>, DynamicStdError>
    {
        let tenants = self.tenants.read().unwrap();

        if let Some(connection) = tenants.get(tenant_id).cloned() {
            debug!("Retrieving ({}) sqlite connection.", tenant_id);
            Ok(Option::from(connection))
        } else {
            warn!(
                "Attempted to retrieve ({}) sqlite connection but it was not found in cache.",
                tenant_id
            );
            Ok(None)
        }
    }

    /// Gets the current amount of tenants in the database.
    pub fn tenant_size(&self) -> usize
    {
        self.tenants.read().unwrap().len()
    }

    /// Creates the master database if none exist yet.
    fn init_master_db(conn: &mut Connection) -> SQLResult<(), MultiTenantError>
    {
        let tx = conn.transaction()?;
        
        tx.execute(SqlStatement::CreateMasterDb.as_str(), [])?;

        if let Err(err) = tx.commit() {
            debug!("Failed to commit transaction: {}", err);
            return Err(MultiTenantError::DatabaseError(format!("Failed to commit transaction: {}", err)));
        }

        Ok(())
    }

    /// Loads the current database tenants into memory. This way developers can get their handles.
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
        let master_db_path = temp_dir.path().join("master.sqlite");
        let _ = MultiTenantManager::new(Configuration {
            master_db_path: Some(master_db_path.clone()),
            log_level: None,
            log_dir: None,
        });
        assert!(master_db_path.exists(), "master.sqlite file does not exist");
    }

    #[test]
    fn test_add_and_remove_tenants()
    {
        let mut manager = MultiTenantManager::new(Configuration {
            master_db_path: None,
            log_level: None,
            log_dir: None,
        })
        .unwrap();

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

    #[test]
    fn test_logger_initialization()
    {
        let config_none = Configuration {
            master_db_path: None,
            // log_level: Some(LogLevel::Trace),
            // Disabled while running other test to avoid making many log files when not needed.
            log_level: None,
            log_dir: None,
        };

        let manager_none = MultiTenantManager::new(config_none).unwrap();

        // assert!(manager_none.logger.is_some(), "Logger should be Some");
        assert!(manager_none.logger.is_none(), "Logger should be None");
    }

    #[test]
    fn test_sql_query()
    {
        let mut manager = MultiTenantManager::new(Configuration {
            master_db_path: None,
            log_level: None,
            log_dir: None,
        })
        .unwrap();

        manager.add_tenant("company-1", None).unwrap();

        match manager.add_tenant("company-1", None) {
            Ok(_) => {}
            Err(err) => {
                assert_eq!(err, MultiTenantError::TenantAlreadyExists("company-1".to_string()))
            }
        }

        manager.add_tenant("company-2", None).unwrap();

        assert_eq!(2, manager.tenant_size());

        #[derive(Debug)]
        struct Person
        {
            id: i32,
            #[allow(dead_code)]
            name: String,
        }

        let sql = manager.get_connection("company-1").unwrap().unwrap().connection;

        sql.execute(
            "CREATE TABLE person (
            id   INTEGER PRIMARY KEY,
            name TEXT NOT NULL
        )",
            (),
        )
        .unwrap();

        let mut people: Vec<Person> = Vec::new();

        for i in 0..5 {
            people.push(Person {
                id: i,
                name: "test_user".to_string(),
            })
        }

        let mut stmt = sql.prepare("SELECT id, name FROM person").unwrap();

        let mut person_iter = stmt
            .query_map([], |row| {
                Ok(Person {
                    id: row.get(0)?,
                    name: row.get(1)?,
                })
            })
            .unwrap();

        // Iterate over the person_iter
        for (index, result) in person_iter.by_ref().enumerate() {
            let person = match result {
                Ok(person) => person,
                Err(err) => {
                    // Handle error if there's any while fetching the person
                    panic!("Error fetching person: {}", err);
                }
            };

            // Check if the current index matches the id of the person
            assert_eq!(index as i32, person.id);
        }
    }
}
