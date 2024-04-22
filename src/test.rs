#[cfg(test)]
mod tests
{
    use std::path::Path;

    use crate::prelude::*;

    #[test]
    fn test_tenant_connection_open_in_memory()
    {
        let db = TenantConnection::open(None::<&Path>).unwrap();
        assert_eq!(db.connection.is_busy(), false);
    }

    #[test]
    fn test_multi_tenant_manager_add_remove_tenant()
    {
        let manager = MultiTenantManager::new();

        // Add a tenant
        let tenant_id = "test_tenant";
        manager.add_tenant(tenant_id, None).unwrap();

        // Check if the tenant exists
        assert!(manager.get_connection(tenant_id).unwrap().is_some());

        // Remove the tenant
        manager.remove_tenant(tenant_id).unwrap();

        // Check if the tenant has been removed
        assert!(manager.get_connection(tenant_id).unwrap().is_none());
    }

    #[derive(Debug)]
    struct Person
    {
        id: i32,
        name: String,
        data: Option<Vec<u8>>,
    }

    #[test]
    fn test_multi_tenant_manager_with_two_tenants()
    {
        // Create a new multi-tenant manager
        let manager = MultiTenantManager::new();

        // Add two tenants
        let tenant_id_1 = "tenant1";
        let tenant_id_2 = "tenant2";
        manager.add_tenant(tenant_id_1, None).unwrap();
        manager.add_tenant(tenant_id_2, None).unwrap();

        // Create the person table in each tenant's database
        create_person_table(&manager, tenant_id_1).unwrap();
        create_person_table(&manager, tenant_id_2).unwrap();

        // Insert data into the first tenant's database
        let conn1 = manager.get_connection(tenant_id_1).unwrap().unwrap();
        insert_person(&conn1, 1, "Alice").unwrap();

        // Insert data into the second tenant's database
        let conn2 = manager.get_connection(tenant_id_2).unwrap().unwrap();
        insert_person(&conn2, 2, "Bob").unwrap();

        // Query data from the first tenant's database
        let mut stmt1 = conn1.connection.prepare("SELECT id, name, data FROM person").unwrap();

        let person_iter1 = stmt1
            .query_map([], |row| {
                Ok(Person {
                    id: row.get(0).unwrap(),
                    name: row.get(1).unwrap(),
                    data: row.get(2).unwrap(),
                })
            })
            .unwrap();

        // Assert that the inserted data for the first tenant is present
        let mut found_person_1 = false;
        for person in person_iter1 {
            let person = person.unwrap();
            if person.id == 1 && person.name == "Alice" {
                found_person_1 = true;
                break;
            }
        }
        assert!(found_person_1, "Failed to find Alice in tenant1");

        // Query data from the second tenant's database
        let mut stmt2 = conn2.connection.prepare("SELECT id, name, data FROM person").unwrap();
        let person_iter2 = stmt2
            .query_map([], |row| {
                Ok(Person {
                    id: row.get(0).unwrap(),
                    name: row.get(1).unwrap(),
                    data: row.get(2).unwrap(),
                })
            })
            .unwrap();

        // Assert that the inserted data for the second tenant is present
        let mut found_person_2 = false;
        for person in person_iter2 {
            let person = person.unwrap();
            if person.id == 2 && person.name == "Bob" {
                found_person_2 = true;
                break;
            }
        }
        assert!(found_person_2, "Failed to find Bob in tenant2");
    }

    // Helper function to insert a person into the database
    fn insert_person(conn: &TenantConnection, id: i32, name: &str) -> SQLResult<()>
    {
        conn.connection
            .execute("INSERT INTO person (id, name) VALUES (?1, ?2)", params![id, name])
            .map(|_| ())
    }

    // Helper function to create the person table in the database if it does not exist
    fn create_person_table(manager: &MultiTenantManager, tenant_id: &str) -> SQLResult<()>
    {
        if let Some(conn) = manager.get_connection(tenant_id).unwrap().as_ref() {
            // Check if the person table exists
            let table_exists: bool = conn
                .connection
                .prepare("SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'person'")
                .and_then(|mut stmt| stmt.query_row([], |_| Ok(true)))
                .unwrap_or(false);

            // If the table does not exist, create it
            if !table_exists {
                conn.connection.execute(
                    "CREATE TABLE person (
                    id   INTEGER PRIMARY KEY,
                    name TEXT NOT NULL,
                    data BLOB
                )",
                    [],
                )?;
            }
        }
        Ok(())
    }
}
