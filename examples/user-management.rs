use std::io;
use std::path::PathBuf;

use sqlite_tenant::prelude::*;

fn main()
{
    let mut manager = MultiTenantManager::new(Configuration {
        master_db_path: Some(PathBuf::new().join("./examples/db/master.sqlite")),
        log_level: Some(LogLevel::Debug),
        log_dir: None,
    })
    .expect("Failed to initialize multi-tenant manager");

    if manager.get_connection("user_db1").unwrap().is_none() {
        manager
            .add_tenant("user_db1", Some(PathBuf::new().join("./examples/db/user_db1.sqlite")))
            .expect("Failed to add user database 1");
    }

    if manager.get_connection("user_db2").unwrap().is_none() {
        manager
            .add_tenant("user_db2", Some(PathBuf::new().join("./examples/db/user_db2.sqlite")))
            .expect("Failed to add user database 2");
    }

    println!("Welcome to the User Management Console!");
    print_help_msg();

    loop {
        let mut input = String::new();
        io::stdin().read_line(&mut input).expect("Failed to read input");

        match input.trim() {
            "1" => handle_user_action(&manager, "user_db1", UserAction::Add),
            "2" => handle_user_action(&manager, "user_db2", UserAction::Add),
            "q" => break,
            "f1" => handle_user_action(&manager, "user_db1", UserAction::Find),
            "f2" => handle_user_action(&manager, "user_db2", UserAction::Find),
            "h" => print_help_msg(),
            _ => println!("Invalid input, please try again"),
        }
    }
}

enum UserAction
{
    Add,
    Find,
}

fn handle_user_action(manager: &MultiTenantManager, db_name: &str, action: UserAction)
{
    match action {
        UserAction::Add => add_user(manager, db_name),
        UserAction::Find => find_user(manager, db_name),
    }
}

fn add_user(manager: &MultiTenantManager, db_name: &str)
{
    println!("Enter username:");
    let username = read_input("Failed to read username");

    println!("Enter email:");
    let email = read_input("Failed to read email");

    match manager.get_connection(db_name) {
        Ok(Some(tenant)) => {
            create_user_db(&tenant.connection);
            let conn = &tenant.connection;

            conn.execute("INSERT INTO users (username, email) VALUES (?1, ?2)", &[&username, &email])
                .expect("Failed to insert user data");

            println!("User added to {} database:", db_name);
            println!("Username: {}", username);
            println!("Email: {}", email);
        }
        Ok(None) => println!("Database '{}' not found", db_name),
        Err(err) => println!("Error: {}", err),
    }
}

fn find_user(manager: &MultiTenantManager, db_name: &str)
{
    println!("Enter username to search:");
    let username = read_input("Failed to read username");

    match manager.get_connection(db_name) {
        Ok(Some(tenant)) => {
            let conn = &tenant.connection;
            create_user_db(&conn);

            let mut stmt = conn
                .prepare("SELECT * FROM users WHERE username = ?1")
                .expect("Failed to prepare statement");
            let user_iter = stmt
                .query_map(&[&username], |row| {
                    Ok((
                        row.get::<usize, i64>(0)?,    // Assuming the first column is id
                        row.get::<usize, String>(1)?, // Assuming the second column is username
                        row.get::<usize, String>(2)?, // Assuming the third column is email
                    ))
                })
                .expect("Failed to execute query");

            let mut user_found = false;

            for user in user_iter {
                let user = user.expect("Failed to fetch user");
                println!("User found in '{}':", db_name);
                println!("ID: {}", user.0);
                println!("Username: {}", user.1);
                println!("Email: {}", user.2);
                user_found = true;
            }

            if !user_found {
                println!("User '{}' not found in '{}'", username, db_name);
            }
        }
        Ok(None) => println!("Database '{}' not found", db_name),
        Err(err) => println!("Error: {}", err),
    }
}

fn create_user_db(conn: &Connection)
{
    conn.execute(
        "CREATE TABLE IF NOT EXISTS users (
            id INTEGER PRIMARY KEY,
            username TEXT NOT NULL,
            email TEXT NOT NULL
        )",
        [],
    )
    .expect("Failed to create user table");
}

fn print_help_msg()
{
    println!("Enter '1' to add a user to database 1");
    println!("Enter '2' to add a user to database 2");
    println!("Enter 'f1' to find a user by username in database 1");
    println!("Enter 'f2' to find a user by username in database 2");
    println!("Enter 'h' for help.");
    println!("Enter 'q' to quit");
}

fn read_input(prompt: &str) -> String
{
    let mut input = String::new();
    io::stdin().read_line(&mut input).expect(prompt);
    input.trim().to_string()
}
