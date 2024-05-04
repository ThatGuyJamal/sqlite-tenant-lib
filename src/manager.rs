extern crate lru;

use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use flexi_logger::{FileSpec, Logger};
use log::{debug, error, info, warn};
use lru::LruCache;
use rusqlite::{params, Connection};

use crate::config::Configuration;
use crate::error::{MultiTenantError, SQLResult};
use crate::statements::SqlStatement;
use crate::tenant::TenantConnection;

type TenantId = String;

pub struct MultiTenantManager
{
    /// The master database manages all the data for other tenants such as lookups, permissions, etc.
    pub(crate) master_db: Connection,
    pub(crate) cache: LruCache<TenantId, TenantConnection>,
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

        MultiTenantManager::init_master_db(&mut master_db).expect("Failed to init master database");

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
            cache: LruCache::new(NonZeroUsize::new(config.lru_cache_cap.unwrap_or(150)).unwrap()),
        })
    }

    /// Adds a new tenant to the manager
    ///
    /// `tenant_id` - used to track a connection to a sqlite db. ID generation should be handled by the library user.
    ///
    /// `path` - to the db file. If `None` is passed, the tenant will be created as an in-memory database.
    pub fn add_tenant(&mut self, tenant_id: &str, path: Option<PathBuf>) -> SQLResult<(), MultiTenantError>
    {
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
        self.cache.put(tenant_id.to_string(), connection);

        info!("Added ({}) tenant.", tenant_id);

        Ok(())
    }

    /// Removes a tenant connection from the manager
    pub fn remove_tenant(&mut self, tenant_id: &str) -> SQLResult<(), MultiTenantError>
    {
        if let Some(tenant) = self.cache.pop(tenant_id) {
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
    pub fn get_connection(&mut self, tenant_id: &str) -> SQLResult<Option<TenantConnection>, MultiTenantError>
    {
        if let Some(connection) = self.cache.get_mut(tenant_id) {
            debug!("Retrieving ({}) sqlite connection from cache.", tenant_id);
            Ok(Some(connection.clone()))
        } else {
            warn!(
                "Attempted to retrieve ({}) sqlite connection but it was not found in cache... searching database...",
                tenant_id
            );

            // If connection not found in cache, search the database
            match Self::load_tenant_from_db(&mut self.master_db, tenant_id) {
                Ok(connection) => {
                    self.cache.put(tenant_id.to_string(), connection.clone());
                    debug!("Retrieving ({}) sqlite connection from database.", tenant_id);
                    Ok(Some(connection))
                }
                Err(e) => Err(e),
            }
        }
    }

    /// Gets the current amount of tenants in the database.
    pub fn tenant_count(&self) -> usize
    {
        self.master_db
            .query_row::<usize, _, _>(SqlStatement::SelectTenantCounts.as_str(), [], |row| row.get(0))
            .unwrap_or_else(|err| {
                error!("Error retrieving tenant count: {}", err);
                0 // Return 0 if there's an error
            })
    }

    /// Creates the master database if none exist yet.
    fn init_master_db(conn: &mut Connection) -> SQLResult<()>
    {
        let tx = conn.transaction()?;

        tx.execute(SqlStatement::CreateMasterDb.as_str(), [])?;
        tx.commit()?;

        Ok(())
    }

    /// Load a tenant connection from the database
    fn load_tenant_from_db(master_db: &mut Connection, tenant_id: &str) -> SQLResult<TenantConnection, MultiTenantError>
    {
        let mut statement = master_db.prepare(SqlStatement::SelectTenant.as_str())?;
        let mut rows = statement.query(params![tenant_id])?;

        if let Some(row) = rows.next()? {
            let path: Option<String> = row.get(0)?;
            let has_path: bool = row.get(1)?;

            let connection = if has_path {
                TenantConnection::open(path.map(PathBuf::from))?
            } else {
                TenantConnection::open(None::<&Path>)?
            };

            debug!("found {} in the database...", tenant_id);

            Ok(connection)
        } else {
            warn!("Tenant ({}) not found in database.", tenant_id);
            Err(MultiTenantError::TenantNotFound(format!(
                "Tenant ({}) not found in database.",
                tenant_id
            )))
        }
    }
}
