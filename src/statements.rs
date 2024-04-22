// https://www.sqlite.org/datatype3.html

pub(crate) enum SqlStatement
{
    CreateMasterDb,
    SelectTenantsOnLoad,
}

impl SqlStatement
{
    pub(crate) fn as_str(&self) -> &'static str
    {
        match self {
            SqlStatement::CreateMasterDb => {
                "
                CREATE TABLE IF NOT EXISTS tenants (
                    id TEXT PRIMARY KEY,
                    tenant_id TEXT,
                    tenant_path TEXT,
                    tenant_has_path INTEGER,
                    created_at TEXT
                )"
            }
            SqlStatement::SelectTenantsOnLoad => "SELECT id, tenant_path, tenant_has_path FROM tenants",
        }
    }
}
