use models::VotingRequest;
use rocket::fs::FileServer;
use rocket::fs::NamedFile;
use rocket::http::Status;
use rocket::response::status;
use rocket::serde::json::Json;
use validator::Validate;

pub mod models;
pub mod repository;

#[macro_use]
extern crate rocket;

// Serve the webpage
#[get("/")]
async fn index() -> Result<NamedFile, std::io::Error> {
    NamedFile::open("static/index.html").await
}

#[post("/user/<username>")]
async fn create_user(username: String) -> Result<(), status::Custom<&'static str>> {
    let result = repository::create_user(username);
    match result {
        Ok(v) => Ok(()),
        Err(e) => {
            println!("{e}");
            Err(status::Custom(
                Status::InternalServerError,
                "Failed to create user by username.",
            ))
        }
    }
}

#[post("/", data = "<body>")]
async fn create_poll(body: Json<VotingRequest>) -> Status {
    // Validate entries
    if let Err(e) = body.validate() {
        Status::BadRequest
    } else {
        // TODO:
        // Implement link generation for accessing the poll (perhaps use dashmap)
        // Implement task spawning which starts a countdown
        // Implement invalidation mechanism after countdown hits 0
        Status::Ok
    }
}

#[launch]
async fn rocket() -> _ {
    let figment = rocket::Config::figment()
        .merge(("address", "0.0.0.0"))
        .merge((
            "port",
            std::env::var("PORT")
                .ok()
                .and_then(|p| p.parse::<u16>().ok())
                .unwrap_or(8080),
        ));

    let do_init = std::env::var("DO_INIT")
        .ok()
        .and_then(|v| v.parse::<bool>().ok())
        .unwrap_or(true);

    if do_init {
        repository::init_db().expect("Failed to initialize DB");
    }

    // create a dashmap which has a Uuid
    rocket::custom(figment)
        .mount("/", routes![create_poll, create_user, index])
        .mount("/static", FileServer::from("static"))
}
