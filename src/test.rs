#[cfg(test)]
mod tests
{
    use flexi_logger::Logger;
    use tempfile::tempdir;

    use crate::prelude::*;

    #[test]
    fn test_master_db_setup()
    {
        let temp_dir = tempdir().expect("Failed to create temporary directory");
        let master_db_path = temp_dir.path().join("master.sqlite");
        let _ = MultiTenantManager::new(Configuration {
            master_db_path: Some(master_db_path.clone()),
            log_level: None,
            log_dir: None,
            lru_cache_cap: None,
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
            lru_cache_cap: None,
        })
        .unwrap();

        // Add 3 tenants
        manager.add_tenant("tenant1", None).expect("Failed to add tenant1");
        manager.add_tenant("tenant2", None).expect("Failed to add tenant2");
        manager.add_tenant("tenant3", None).expect("Failed to add tenant3");

        assert_eq!(manager.tenant_count(), 3);
    }

    #[test]
    fn test_sql_query()
    {
        let temp_dir = tempdir().expect("Failed to create temporary directory");

        let mut manager = MultiTenantManager::new(Configuration {
            master_db_path: Some(temp_dir.path().join("master.sqlite")),
            log_level: None,
            log_dir: None,
            lru_cache_cap: None,
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

        assert_eq!(3, manager.tenant_count());

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

    #[test]
    fn test_logger_configuration()
    {
        // Create a temporary directory for logs
        let temp_dir = tempdir().expect("Failed to create temporary directory");

        // Set up test configuration
        let config = Configuration {
            master_db_path: None,
            log_level: Some(LogLevel::Debug), // Set log level to debug for testing
            log_dir: Some(temp_dir.path().join("logs")),
            lru_cache_cap: None,
        };

        // Create a new logger based on the test configuration
        let logger = if let Some(log_level) = config.log_level {
            Some(
                Logger::try_with_str(log_level.as_str())
                    .unwrap()
                    .log_to_file(flexi_logger::FileSpec::default().directory(config.log_dir.unwrap()))
                    .duplicate_to_stdout(log_level.as_dup())
                    .start()
                    .unwrap(),
            )
        } else {
            None
        };

        // Assert that the logger is correctly created
        assert!(logger.is_some());
    }
}
