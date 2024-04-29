use std::error::Error;
use std::fmt;

pub type SQLError = rusqlite::Error;
pub type DynamicStdError = Box<dyn Error>;
pub type SQLResult<T, E = SQLError> = Result<T, E>;

#[derive(Debug, PartialEq)]
pub enum MultiTenantError
{
    TenantAlreadyExists(String),
    TenantNotFound(String),
    DatabaseError(String),
}

impl Error for MultiTenantError {}

impl fmt::Display for MultiTenantError
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self {
            MultiTenantError::TenantAlreadyExists(tenant_id) => {
                write!(f, "Tenant '{}' already exists", tenant_id)
            }
            MultiTenantError::TenantNotFound(tenant_id) => {
                write!(f, "Tenant '{}' not found", tenant_id)
            }
            MultiTenantError::DatabaseError(msg) => write!(f, "Database error: {}", msg),
        }
    }
}

impl From<rusqlite::Error> for MultiTenantError
{
    fn from(err: rusqlite::Error) -> Self
    {
        match err {
            rusqlite::Error::QueryReturnedNoRows => MultiTenantError::TenantNotFound("No row data found.".to_string()),
            rusqlite::Error::SqliteFailure(_, msg) => {
                if let Some(msg) = msg {
                    MultiTenantError::DatabaseError(msg.to_string())
                } else {
                    MultiTenantError::DatabaseError("Failed to get database error message.".to_string())
                }
            }
            // todo - handle more errors
            _ => MultiTenantError::DatabaseError("Unknown database error".to_string()),
        }
    }
}
