// https://www.sqlite.org/datatype3.html

/// SQL statements used in the tenant manager.
pub(crate) enum SqlStatement
{
    CreateMasterDb,
    SelectTenantsOnLoad,
    InsertAddTenant,
    DeleteRemoveTenant,
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
                )"
            }
            SqlStatement::SelectTenantsOnLoad => "SELECT tenant_id, tenant_path, tenant_has_path FROM tenants",
            SqlStatement::InsertAddTenant => {
                "INSERT INTO tenants (tenant_id, tenant_path, tenant_has_path) VALUES (?1, ?2, ?3)"
            }
            SqlStatement::DeleteRemoveTenant => "DELETE FROM tenants WHERE id = ?1",
        }
    }
}
