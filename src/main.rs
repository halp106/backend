mod app_logic;

#[macro_use] extern crate rocket;

use rocket::http::Header;
use rocket::{Request, Response, State};
use rocket::fairing::{Fairing, Info, Kind};
use rocket::request::{FromRequest, Outcome};
use rocket::serde::{Serialize, Deserialize, json::Json};
use rocket::serde::json::serde_json::json;
use rocket::http::Status;

// Set up CORS
pub struct CORS;

#[rocket::async_trait]
impl Fairing for CORS {
    fn info(&self) -> Info {
        Info {
            name: "Add CORS headers to responses",
            kind: Kind::Response
        }
    }

    async fn on_response<'r>(&self, _request: &'r Request<'_>, _response: &mut Response<'r>) {
        // Set up response headers
        // Note: These settings are _VERY_ permissive. They'll work for the demo though.
        _response.set_header(Header::new("Access-Control-Allow-Origin", "*"));
        _response.set_header(Header::new("Access-Control-Allow-Methods", "POST, GET, DELETE, PATCH, OPTIONS"));
        _response.set_header(Header::new("Access-Control-Allow-Headers", "*"));
        _response.set_header(Header::new("Access-Control-Allow-Credentials", "true"));
    }
}

// Data Structs
struct DbState {
    in_memory: bool,
    db_path: String,
}

#[derive(Deserialize)]
struct RegisterInfo<'r> {
    username: &'r str,
    email: &'r str,
    password: &'r str,
}

#[derive(Deserialize)]
struct LoginInfo<'r> {
    username: &'r str,
    password: &'r str,
}

#[derive(Serialize)]
struct Post<'r> {
    unique_id: &'r str,
    title: &'r str,
    author_username: &'r str,
    timestamp: &'r str,
    tag: &'r str,
    content: &'r str,
}

// Request Guards
pub struct AuthenticationKey {
    key_content: String,
}

#[derive(Debug)]
pub enum AuthenticationKeyError {
    Missing,
    Invalid,
    DbError
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for AuthenticationKey {
    type Error = AuthenticationKeyError;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        // Retrieve DB managed state
        let db_state = match request.rocket().state::<DbState>() {
            Some(res) => res,
            None => return Outcome::Failure((Status::InternalServerError, AuthenticationKeyError::DbError)),
        };

        // Get the authentication key from the request header
        let auth_key = match request.headers().get_one("x-auth-key") {
            Some(res) => res.to_string(),
            None => return Outcome::Failure((Status::Unauthorized, AuthenticationKeyError::Missing)),
        };

        // Get a connection to the database
        let mut conn = match app_logic::connect_db(&db_state.db_path, db_state.in_memory) {
            Ok(res) => res,
            Err(_) => return Outcome::Failure((Status::InternalServerError, AuthenticationKeyError::DbError)),
        };

        // Check the authentication key against the database and return
        match app_logic::authenticate(&mut conn, &auth_key) {
            Ok(res) => match res {
                true => Outcome::Success(AuthenticationKey {
                    key_content: auth_key
                }),
                false => Outcome::Failure((Status::Unauthorized, AuthenticationKeyError::Invalid)),
            },
            Err(_) => Outcome::Failure((Status::InternalServerError, AuthenticationKeyError::DbError)),
        }
    }
}

// Routing & Handlers
#[get("/")]
fn index() -> &'static str {
    "Hello, world!"
}

#[post("/register", data="<input>")]
fn register(input: Json<RegisterInfo<'_>>, db_state: &State<DbState>) -> rocket::serde::json::Value {
    // Variable to store whether or not the user registration was successful
    let mut success = false;

    // Connect to the DB
    let mut conn = match app_logic::connect_db(&db_state.db_path, db_state.in_memory) {
        Ok(val) => val,
        Err(e) => {
            println!("Encountered an error while connecting to the DB to register: {}", e);
            success = false;
            panic!("Panicked while trying to connect to DB to register!")
        }
    };

    // Create the user
    success = match app_logic::create_user(
        &mut conn,
        &input.username.to_string(),
        &input.email.to_string(),
        &input.password.to_string()
    ) {
        Ok(_) => true,
        Err(e) => {
            println!("Encountered an error while creating the user in the DB: {}", e);
            false
        }
    };

    // Return JSON (if we get this far!)
    json!({
        "success": success
    })
}

#[post("/login", data="<input>")]
fn login(input: Json<LoginInfo<'_>>, db_state: &State<DbState>) -> rocket::serde::json::Value {
    // Connect to the DB
    let mut conn = match app_logic::connect_db(&db_state.db_path, db_state.in_memory) {
        Ok(val) => val,
        Err(e) => {
            println!("Encountered an error while connecting to the DB to register: {}", e);
            panic!("Panicked while trying to connect to DB to log the user in!")
        }
    };

    let authentication_key = match app_logic::login(&mut conn, &String::from(input.username), &String::from(input.password)) {
        Ok(val) => val,
        Err(e) => {
            println!("Encountered an error while trying to authenticate the user: {}", e);
            panic!("Panicked while trying to log the user in!")
        }
    };

    // Return the authentication key for this user
    json!({
        "auth_key": authentication_key.0,
        "expiration_datetime": authentication_key.1
    })
}

#[get("/posts")]
fn get_posts(authentication_key: AuthenticationKey) -> &'static str {
    let _ = app_logic::test_db();

    "Implement me!"
}

// Launch
#[launch]
fn rocket() -> _ {
    // Init variables
    let db_state = DbState {
        in_memory: false,
        db_path: String::from("./test_db.sqlite"),
    };

    // Run initial setup & get DB connection
    let mut db_conn = app_logic::connect_db(&db_state.db_path, db_state.in_memory).unwrap();
    app_logic::setup_database(&mut db_conn).unwrap();

    // Run Rocket setup
    rocket::build()
        .manage(db_state)  // Manage DB state
        .attach(CORS)
        .mount("/", routes![index, register, login, get_posts])
}