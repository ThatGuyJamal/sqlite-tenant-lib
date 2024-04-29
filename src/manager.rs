use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use flexi_logger::{FileSpec, Logger};
use log::{debug, error, info, warn};
use rusqlite::{params, Connection, Statement, ToSql};

use crate::config::Configuration;
use crate::error::{DynamicStdError, MultiTenantError, SQLResult};
use crate::statements::SqlStatement;
use crate::tenant::TenantConnection;

type TenantId = String;

pub struct MultiTenantManager
{
    /// The master database manages all the data for other tenants such as lookups, permissions, etc.
    pub(crate) master_db: Connection,
    pub(crate) tenants: Arc<RwLock<HashMap<TenantId, TenantConnection>>>,
}

impl MultiTenantManager
{
    /// Created a new tenant manager.
    pub fn new(config: Configuration) -> SQLResult<Self>
    {
        let mut master_db = match config.master_db_path {
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

        // Set up the logger settings for the manager
        if let Some(log_level) = config.log_level {
            // If log_level is Some, initialize the logger
            match Logger::try_with_str(log_level.as_str()) {
                Ok(logger_builder) => {
                    match logger_builder
                        .log_to_file(FileSpec::default().directory(config.log_dir.unwrap_or(PathBuf::from("logs"))))
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
            return Err(MultiTenantError::DatabaseError(format!(
                "Failed to commit transaction: {}",
                err
            )));
        }

        let connection = TenantConnection::open(path.clone())?;
        tenants.insert(tenant_id.to_string(), connection);

        info!("Added ({}) tenant.", tenant_id);

        Ok(())
    }

    /// Removes a tenant connection from the manager
    pub fn remove_tenant(&mut self, tenant_id: &str) -> SQLResult<(), MultiTenantError>
    {
        let mut tenants = self.tenants.write().unwrap();

        if let Some(tenant) = tenants.remove(tenant_id) {
            // Close the connection held within the Arc
            Arc::try_unwrap(tenant.connection)
                .map_err(|_| MultiTenantError::DatabaseError(format!("Failed to unwrap Arc for {}", tenant_id)))?
                .close()
                .map_err(|e| {
                    MultiTenantError::DatabaseError(format!("Failed to close connection for {}: {:?}", tenant_id, e))
                })?;

            // Begin a transaction
            let tx = self.master_db.transaction()?;

            tx.execute(SqlStatement::DeleteRemoveTenant.as_str(), params![tenant_id])?;

            if let Err(err) = tx.commit() {
                debug!("Failed to commit transaction: {}", err);
                return Err(MultiTenantError::DatabaseError(format!(
                    "Failed to commit transaction: {}",
                    err
                )));
            }

            debug!("Deleted ({}) tenant.", tenant_id);
            Ok(())
        } else {
            error!("Attempted to delete tenant ({}) that does not exist.", tenant_id);
            Err(MultiTenantError::TenantNotFound(tenant_id.to_string()))
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
            return Err(MultiTenantError::DatabaseError(format!(
                "Failed to commit transaction: {}",
                err
            )));
        }

        Ok(())
    }

    /// Loads the current database tenants into memory. This way developers can get their handles.
    fn load_tenants(master_db: &Connection) -> SQLResult<HashMap<TenantId, TenantConnection>>
    {
        let mut statement: Statement = master_db.prepare(SqlStatement::SelectTenantsOnLoad.as_str())?;

        // see - https://docs.rs/rusqlite/0.31.0/rusqlite/trait.Params.html#dynamic-parameter-list
        let rows = statement.query_map::<_, &[&dyn ToSql], _>(&[], |row| {
            let tenant_id: String = row.get(0)?;
            let tenant_path: Option<String> = row.get(1)?;
            let tenant_has_path: bool = row.get(2)?;

            let connection: TenantConnection = if tenant_has_path {
                TenantConnection::open(Some(tenant_path.expect("Expected path, but found None")))?
            } else {
                TenantConnection::open(None::<&Path>)?
            };

            Ok((tenant_id, connection))
        })?;

        let mut tenants = HashMap::<TenantId, TenantConnection>::new();
        for result in rows {
            let (tenant_id, connection) = result?;
            tenants.insert(tenant_id, connection);
        }

        debug!("Loaded {} tenants into cache from the master db.", tenants.len());

        Ok(tenants)
    }
}
