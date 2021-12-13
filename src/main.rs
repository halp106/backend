#[macro_use] extern crate rocket;

use rocket::http::Header;
use rocket::{Build, Data, Orbit, Request, Response, Rocket};
use rocket::fairing::{Fairing, Info, Kind};

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

// Routing & Handlers
#[get("/")]
fn index() -> &'static str {
    "Hello, world!"
}

#[post("/login")]
fn login() -> &'static str {

    "Login attempt"
}

// Launch
#[launch]
fn rocket() -> _ {
    rocket::build().attach(CORS).mount("/", routes![index, login])
}