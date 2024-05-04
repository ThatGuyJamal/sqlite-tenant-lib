// https://www.sqlite.org/datatype3.html

#[allow(dead_code)]
#[derive(Debug)]
/// Rust type representation of our SQL master table.
pub(crate) struct MasterDbTable
{
    id: String,
    tenant_id: String,
    tenant_path: Option<String>,
    tenant_has_path: i64, // 0 = false, 1 = true
    created_at: String,
}

/// SQL statements used in the tenant manager.
pub(crate) enum SqlStatement
{
    CreateMasterDb,
    InsertAddTenant,
    DeleteRemoveTenant,
    SelectTenant,
    SelectTenantCounts,
}

impl SqlStatement
{
    pub(crate) fn as_str(&self) -> &'static str
    {
        match self {
            SqlStatement::CreateMasterDb => {
                "
                CREATE TABLE IF NOT EXISTS tenants (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    tenant_id TEXT NOT NULL,
                    tenant_path TEXT,
                    tenant_has_path INTEGER NOT NULL DEFAULT 0,
                    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
                );"
            }
            SqlStatement::InsertAddTenant => {
                "INSERT INTO tenants (tenant_id, tenant_path, tenant_has_path) VALUES (?1, ?2, ?3);"
            }
            SqlStatement::DeleteRemoveTenant => "DELETE FROM tenants WHERE id = ?1;",
            SqlStatement::SelectTenant => "SELECT tenant_path, tenant_has_path FROM tenants WHERE tenant_id = ?1;",
            SqlStatement::SelectTenantCounts => "SELECT COUNT(*) FROM tenants;",
        }
    }
}
