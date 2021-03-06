use std::path::Path;
use rusqlite::{params, Connection};
use chrono::{DateTime, Duration, Utc};
use argon2::{self, Config};
use rand::{distributions::Alphanumeric, Rng};
use rocket::serde::{Serialize, json::Json};

// Structures
#[derive(Debug)]
struct TestEntry {
    id: i32,
    text: String,
}

struct User {
    unique_id: isize,
    username: String,
    email: String,
    password_hash: String,
    registration_datetime: String,
}

#[derive(Serialize)]
pub struct Thread {
    unique_id: isize,
    title: String,
    creator_uid: isize,
    creation_timestamp: String,
    tag: String,
    content: String,
}

#[derive(Serialize)]
pub struct Comment {
    unique_id: isize,
    thread_id: isize,
    creator_uid: isize,
    creation_timestamp: String,
    content: String,
}

// Testing and Debugging
pub fn test_db() -> rusqlite::Result<()> {
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

// App Logic Functions
pub fn connect_db(path: &String, in_memory: bool) -> rusqlite::Result<Connection> {
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

pub fn setup_database(conn: &mut Connection) -> rusqlite::Result<()> {

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

    // Return success if everything completes
    Ok(())
}

pub fn authenticate(conn: &mut Connection, auth_key: &String) -> rusqlite::Result<bool> {
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
        let expiration: String = row.get(0)?;

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

pub fn reverse_key_lookup(conn: &mut Connection, auth_key: &String) -> rusqlite::Result<String> {
    // Prepare query in the DB that retrieves all matching authentication keys
    let mut reverse_key_lookup_query = match conn.prepare("SELECT \
                        user_id \
                      FROM \
                        authentication_keys \
                      WHERE \
                        authentication_key = ?1"
    ) {
        Ok(val) => val,
        Err(e) => {
            println!("Error encountered while running query for reverse key lookup: {}", e);
            panic!("Panicked after not being able to resolve error!")
        }
    };

    // Iterate through results to find matching user_id
    let reverse_key_lookup_results = reverse_key_lookup_query.query_map(params![auth_key], |row| {
        // Get values from the row as String objects
        let user_id: isize = row.get(0)?;
        Ok(user_id.to_string())
    })?;

    // Actually run iterator to find the matching user_id
    for entry in reverse_key_lookup_results {
        return Ok(entry?)
    }

    // If nothing was found
    Err(rusqlite::Error::InvalidQuery)
}

pub fn get_username_from_uid(conn: &mut Connection, unique_id: &String) -> rusqlite::Result<String> {
    // Prepare query in the DB that retrieves all matching authentication keys
    let mut matching_usernames_query = match conn.prepare("SELECT \
                        username \
                      FROM \
                        users \
                      WHERE \
                        unique_id = ?1"
    ) {
        Ok(val) => val,
        Err(e) => {
            println!("Error encountered while running query for user based on uid: {}", e);
            panic!("Panicked after not being able to resolve error!")
        }
    };

    // Iterate through results to find matching username
    let matching_username_results = matching_usernames_query.query_map(params![unique_id], |row| {
        // Get values from the row as String objects
        let username: String = row.get(0)?;
        Ok(username)
    })?;

    // Actually run iterator to find the matching user_id
    for entry in matching_username_results {
        return Ok(entry?)
    }

    // If nothing was found
    Err(rusqlite::Error::InvalidQuery)
}

pub fn get_uid_from_username(conn: &mut Connection, username: &String) -> rusqlite::Result<String> {
    // Prepare query in the DB that retrieves all matching authentication keys
    let mut matching_uids_query = match conn.prepare("SELECT \
                        unique_id \
                      FROM \
                        users \
                      WHERE \
                        username = ?1"
    ) {
        Ok(val) => val,
        Err(e) => {
            println!("Error encountered while running query for username based on username: {}", e);
            panic!("Panicked after not being able to resolve error!")
        }
    };

    // Iterate through results to find matching uid
    let matching_uid_results = matching_uids_query.query_map(params![username], |row| {
        // Get values from the row as String objects
        let uid: isize = row.get(0)?;
        Ok(uid.to_string())
    })?;

    // Actually run iterator to find the matching user_id
    for entry in matching_uid_results {
        return Ok(entry?)
    }

    // If nothing was found
    Err(rusqlite::Error::InvalidQuery)
}

pub fn login(conn: &mut Connection, username: &String, password: &String) -> rusqlite::Result<(String, String)> {
    // Creates an authentication token for a user given the user's password

    // Create statement that finds the desired user
    let mut user_query_statement = conn.prepare(
        "SELECT \
                unique_id, username, email, password_hash, registration_datetime \
             FROM \
                users \
             WHERE \
                username = ?"
    )?;

    // Create iterator for results
    let row_iter = user_query_statement.query_map(params![username], |row| {
        Ok(User {
            unique_id: row.get(0)?,
            username: row.get(1)?,
            email: row.get(2)?,
            password_hash: row.get(3)?,
            registration_datetime: row.get(4)?
        })
    })?;

    // Iterate through matching users and verify correct password
    let mut matching_uid: Option<String> = None;
    for entry in row_iter {
        let user = entry?;

        // Check if the password matches
        let password_valid = match argon2::verify_encoded(&user.password_hash, (&password).as_ref()) {
            Ok(val) => {
                matching_uid = Some(user.unique_id.to_string());
                val
            },
            Err(e) => {
                println!("Encountered an error while attempting to validate password: {}", e);
                false
            }
        };
    }

    // Create a new authentication key for the user
    let authentication_key: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(32)
        .map(char::from)
        .collect();

    // Generate a new expiration date for the new authentication key
    let expiration_date = Utc::now() + Duration::days(10);

    // Record the authentication key and the expiration date in the DB
    match matching_uid {
        Some(unique_id) => {
            conn.execute(
                "INSERT INTO authentication_keys (user_id, authentication_key, expiration) VALUES (?1, ?2, ?3)",
                params![unique_id, authentication_key, expiration_date.to_rfc3339()]
            )?;
        },
        None => {
            println!("Tried to authenticate for a user that doesn't exist!");
            return Err(rusqlite::Error::InvalidQuery);
        }
    }

    // Return authentication key and the expiration date
    Ok((authentication_key, expiration_date.to_rfc3339()))
}

pub fn get_threads(conn: &mut Connection) -> rusqlite::Result<Vec<Thread>> {

    // Craft the SQL query
    let mut threads_query_statement = conn.prepare(
        "SELECT unique_id, title, creator_uid, creation_timestamp, tag, content FROM threads"
    )?;

    // Create iterator to iterate through matching DB rows
    let row_iter = threads_query_statement.query_map([], |row| {
        Ok(Thread {
            unique_id: row.get(0)?,
            title: row.get(1)?,
            creator_uid: row.get(2)?,
            creation_timestamp: row.get(3)?,
            tag: row.get(4)?,
            content: row.get(5)?,
        })
    })?;

    // Vector to store thread structs in
    let mut threads: Vec<Thread> = Vec::new();

    // Iterate through the DB rows
    for entry in row_iter {
        let thread = entry?;
        threads.push(thread);
    }

    // Return the vector of Thread structs
    Ok(threads)
}

pub fn get_thread_comments(conn: &mut Connection, thread_uid: &String) -> rusqlite::Result<Vec<Comment>> {
    // Craft the SQL query
    let mut comments_query_statement = conn.prepare(
        "SELECT unique_id, thread_id, creator_uid, creation_timestamp, content FROM comments WHERE thread_id = ?1"
    )?;

    // Create iterator to iterate through matching DB rows
    let row_iter = comments_query_statement.query_map(params![thread_uid], |row| {
        Ok(Comment {
            unique_id: row.get(0)?,
            thread_id: row.get(1)?,
            creator_uid: row.get(2)?,
            creation_timestamp: row.get(3)?,
            content: row.get(4)?,
        })
    })?;

    // Vector to store comment structs in
    let mut comments: Vec<Comment> = Vec::new();

    // Iterate through the DB rows
    for entry in row_iter {
        let thread = entry?;
        comments.push(thread);
    }

    // Return the vector of Comment structs
    Ok(comments)
}

pub fn create_user(conn: &mut Connection, username: &String, email: &String, password: &String) -> rusqlite::Result<bool> {
    // Get current time (to be registration datetime)
    let now = Utc::now();

    // Generate the salt to use
    let salt: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(10)
        .map(char::from)
        .collect();
    let config = Config::default();

    // Generate the password hash based on the password and the salt
    let hash = argon2::hash_encoded(password.as_ref(), salt.as_str().as_ref(), &config).unwrap();

    // Create the user in the database
    conn.execute(
        "INSERT INTO \
                users (username, email, password_hash, password_salt, registration_datetime) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
        params![username.as_str(), email.as_str(), hash.as_str(), salt, now.to_rfc3339().as_str()]
    )?;

    // If all succeeds, return true
    Ok(true)
}

pub fn create_thread(conn: &mut Connection, title: &String, username: &String, tag: &String, content: &String) -> rusqlite::Result<bool> {
    // Get current time (to be the thread creation timestamp)
    let now = Utc::now();

    // Get the matching UID that corresponds to the user
    let unique_user_id = get_uid_from_username(conn, username)?;

    // Create the user in the database
    conn.execute(
        "INSERT INTO \
                threads (title, creator_uid, creation_timestamp, tag, content) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
        params![title, unique_user_id, now.to_rfc3339(), tag, content]
    )?;

    // If all succeeds, return true
    Ok(true)
}

pub fn create_comment(conn: &mut Connection, thread_uid: &String, username: &String, content: &String) -> rusqlite::Result<bool> {
    // Get current time (to be the thread creation timestamp)
    let now = Utc::now();

    // Get the matching UID that corresponds to the user
    let unique_user_id = get_uid_from_username(conn, username)?;

    // Create the user in the database
    conn.execute(
        "INSERT INTO \
                comments (thread_id, creator_uid, creation_timestamp, content) \
             VALUES (?1, ?2, ?3, ?4)",
        params![thread_uid, unique_user_id, now.to_rfc3339(), content]
    )?;

    // If all succeeds, return true
    Ok(true)
}

pub fn delete_thread(conn: &mut Connection, thread_uid: &String) -> rusqlite::Result<bool> {
    todo!("Implement delete_thread function")
}

pub fn delete_comment(conn: &mut Connection, thread_uid: &String) -> rusqlite::Result<bool> {
    todo!("Implement delete_comment function")
}
