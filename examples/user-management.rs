use std::path::PathBuf;
use std::io;

use sqlite_tenant::prelude::*;

fn main() {
    // Initialize the multi-tenant manager with a configuration
    let mut manager = MultiTenantManager::new(Configuration {
        master_db_path: Some(PathBuf::new().join("./examples/db/master.sqlite")), // Master database path
        log_level: Some(LogLevel::Info), // Set log level to Info
        log_dir: None, // No specific log directory. Defaults to ./logs
    }).expect("Failed to initialize multi-tenant manager");
    
    // Make sure we only create the db if it does not exist
    match manager.get_connection("user_db1").unwrap() {
        None => {
            manager.add_tenant("user_db1", Some(PathBuf::new().join("./examples/db/user_db1.sqlite")))
                .expect("Failed to add user database 1");
        }
        Some(_) => {}
    };

    match manager.get_connection("user_db1").unwrap() {
        None => {
            manager.add_tenant("user_db2", Some(PathBuf::new().join("./examples/db/user_db2.sqlite")))
                .expect("Failed to add user database 2");
        }
        Some(_) => {}
    };

    println!("Welcome to the User Management Console!");
    println!("Enter '1' to add a user to database 1");
    println!("Enter '2' to add a user to database 2");
    println!("Enter 'q' to quit");

    loop {
        let mut input = String::new();
        io::stdin().read_line(&mut input)
            .expect("Failed to read input");

        match input.trim() {
            "1" => add_user(&manager, "user_db1"),
            "2" => add_user(&manager, "user_db2"),
            "q" => break,
            _ => println!("Invalid input, please try again"),
        }
    }
}

fn add_user(manager: &MultiTenantManager, db_name: &str) {
    println!("Enter username:");
    let mut username = String::new();
    io::stdin().read_line(&mut username)
        .expect("Failed to read username");

    // Trim whitespace and newline characters
    let username = username.trim();

    println!("Enter email:");
    let mut email = String::new();
    io::stdin().read_line(&mut email)
        .expect("Failed to read email");

    // Trim whitespace and newline characters
    let email = email.trim();

    // Insert user data into the specified database
    match manager.get_connection(db_name) {
        Ok(Some(_connection)) => {
            // Perform database operation (e.g., insert user data)
            // For demonstration purposes, let's just print the user data
            println!("User added to {} database:", db_name);
            println!("Username: {}", username);
            println!("Email: {}", email);
        }
        Ok(None) => println!("Database '{}' not found", db_name),
        Err(err) => println!("Error: {}", err),
    }
}
