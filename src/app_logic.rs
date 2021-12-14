use std::path::Path;
use rusqlite::{params, Connection, Result};
use chrono::{DateTime, Utc};

#[derive(Debug)]
struct TestEntry {
    id: i32,
    text: String,
}

pub fn test_db() -> Result<()> {
    let conn = Connection::open_in_memory()?;

    conn.execute(
        "CREATE TABLE testing (id INTEGER PRIMARY KEY, text TEXT NOT NULL)",
        []
    )?;

    let test_entry = TestEntry {
        id: 0i32,
        text: String::from("Testing"),
    };

    conn.execute(
        "INSERT INTO testing (id, text) VALUES (?1, ?2)",
        params![test_entry.id, test_entry.text],
    )?;

    let mut stmt = conn.prepare("SELECT id, text FROM testing")?;

    let row_iter = stmt.query_map([], |row| {
        Ok(TestEntry {
            id: row.get(0)?,
            text: row.get(1)?,
        })
    })?;

    for entry in row_iter {
        println!("Found row: {:?}", entry.unwrap());
    }

    Ok(())
}

pub fn connect_db(path: &String, in_memory: bool) -> Result<Connection> {
    // Get path specified in argument as an actual path
    let db_path = Path::new(path);

    // Open a connection to the database
    let conn = match in_memory {
        true => Connection::open_in_memory()?,
        false => Connection::open(db_path)?,
    };

    // Return the connection
    Ok(conn)
}

pub fn setup_database(conn: &mut Connection) -> Result<()> {

    // Create Users table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS users ( \
                unique_id INTEGER PRIMARY KEY, \
                username TEXT UNIQUE NOT NULL, \
                email TEXT UNIQUE, \
                password_hash TEXT NOT NULL, \
                password_salt TEXT NOT NULL, \
                registration_datetime TEXT \
            );",
        []
    )?;

    // Create Authentication Keys table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS authentication_keys ( \
                unique_id INTEGER PRIMARY KEY, \
                user_id INTEGER NOT NULL, \
                authentication_key TEXT NOT NULL, \
                expiration TEXT NOT NULL, \
                FOREIGN KEY (user_id) references users(unique_id) ON DELETE CASCADE \
            );",
        []
    )?;

    // Create the User Privileges table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS user_privileges ( \
                privilege_unique_id INTEGER PRIMARY KEY, \
                user_id INTEGER NOT NULL, \
                privilege TEXT NOT NULL, \
                FOREIGN KEY (user_id) references users(unique_id) ON DELETE CASCADE \
            );",
        []
    )?;

    // Create the Threads/Posts table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS threads ( \
                unique_id INTEGER PRIMARY KEY, \
                title TEXT NOT NULL, \
                creator_uid INTEGER NOT NULL, \
                creation_timestamp TEXT, \
                tag TEXT, \
                content TEXT NOT NULL, \
                FOREIGN KEY (creator_uid) references users(unique_id) ON DELETE CASCADE \
            );",
        []
    )?;

    // Create Comments table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS comments ( \
                unique_id INTEGER PRIMARY KEY, \
                thread_id INTEGER NOT NULL, \
                creator_uid INTEGER NOT NULL, \
                creation_timestamp TEXT, \
                content TEXT NOT NULL, \
                FOREIGN KEY (thread_id) references threads(unique_id) ON DELETE CASCADE, \
                FOREIGN KEY (creator_uid) references users(unique_id) ON DELETE CASCADE \
            );",
        []
    )?;

    // Create the "root" admin user
    // todo!("Add correct SQL to create the 'root' admin user");
    // conn.execute(
    //     "INSERT INTO users ( \
    //             username, \
    //             email, \
    //             password_hash, \
    //             password_salt, \
    //             registration_datetime) \
    //          VALUES (1?, 2?, 3?, 4?, 5?)",
    //     params!["..."]
    // )?;
    //
    // conn.execute(
    //     "INSERT INTO user_privileges (user_id, privilege) VALUES (?1, ?2)",
    //     params!["..."]
    // )?;

    // Return success if everything completes
    Ok(())
}

pub fn authenticate(conn: &mut Connection, auth_key: &String) -> Result<bool> {
    // Verifies a username and authentication token against the database (and expiration datetime)

    // Get the current time
    let now = Utc::now();
    println!("Time now is: {}", now.to_rfc3339());

    // Prepare query in the DB that retrieves all matching authentication keys
    let mut auth_keys_query = match conn.prepare("SELECT \
                        expiration \
                      FROM \
                        authentication_keys \
                      WHERE \
                        authentication_key = ?1"
    ) {
        Ok(val) => val,
        Err(e) => {
            println!("Error encountered while running query for Authenticate: {}", e);
            panic!("Panicked after not being able to resolve error!")
        }
    };

    // Iterate through results and verify that the authentication key has not expired
    // Note: Also checks for lack of auth tokens by never running expiration checks if no matching
    // rows exist
    let mut valid_key_encountered = false;
    let auth_key_results = auth_keys_query.query_map(params![auth_key], |row| {
        // Get values from the row as String objects
        let expiration: String = row.get(1)?;

        // Parse the row values
        let expiration_datetime = match DateTime::parse_from_rfc3339(expiration.as_str()) {
            Ok(val) => val,
            Err(e) => {
                println!("Error when parsing expiration datetime: {}", e);
                panic!("Panicked while parsing expiration datetime!");
            }
        };

        // Check expiration datetime against current time, set flag if still valid
        let duration_since_expiration = now.signed_duration_since(expiration_datetime);
        let mut not_expired_flag = false;
        if duration_since_expiration.num_seconds() < 0 {
            valid_key_encountered = true;
            not_expired_flag = true;
        }

        Ok(not_expired_flag)
    })?;

    // Actually run iterator (and count how many expired authentication keys have been collected)
    let mut expired_tokens = 0;
    for entry in auth_key_results {
        let safe_entry = entry?;
        if safe_entry {
            expired_tokens += 1;
        }
    }
    println!("Found {} expired token(s) while authenticating!", expired_tokens);

    // Return the result of the expiration check
    Ok(valid_key_encountered)
}

pub fn login(conn: &mut Connection, username: &String, password: &String) -> Result<String> {
    // Creates an authentication token for a user given the user's password
    todo!("Implement login function")
}

pub fn get_posts(conn: &mut Connection, ) -> Result<String> {
    todo!("Implement get_posts function")
}

pub fn get_post_comments(conn: &mut Connection, thread_uid: &String) -> Result<String> {
    todo!("Implement get_post_comments function")
}

pub fn create_user(conn: &mut Connection, username: &String, email: &String, password: &String) -> Result<bool> {
    todo!("Implement create_user function")
}

pub fn create_post(conn: &mut Connection, title: &String, username: &String, timestamp: &String, tag: &String, content: &String) -> Result<bool> {
    todo!("Implement create_post function")
}

pub fn create_comment(conn: &mut Connection, thread_uid: &String, username: &String, timestamp: &String, content: &String) -> Result<bool> {
    todo!("Implement create_comment function")
}

pub fn delete_thread(conn: &mut Connection, thread_uid: &String) -> Result<bool> {
    todo!("Implement delete_thread function")
}

pub fn delete_comment(conn: &mut Connection, thread_uid: &String) -> Result<bool> {
    todo!("Implement delete_comment function")
}
