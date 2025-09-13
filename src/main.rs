use std::sync::Arc;

use dashmap::DashMap;
use models::VotingRequest;
use rocket::State;
use rocket::fs::FileServer;
use rocket::fs::NamedFile;
use rocket::http::Status;
use rocket::response::status;
use rocket::serde::json::Json;
use rocket::tokio;
use rocket::tokio::time::{Duration, sleep};
use uuid::Uuid;
use validator::Validate;

use self::models::VotingState;
use self::repository::save_voting_poll;

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

// In construction
#[post("/", data = "<body>")]
async fn create_poll(
    body: Json<VotingRequest>,
    active_polls: &State<Arc<DashMap<String, VotingState>>>,
) -> Result<String, status::Custom<&'static str>> {
    // Validate entries
    if body.validate().is_err() {
        Err(status::Custom(Status::BadRequest, "Validation failed"))
    } else {
        save_voting_poll(body.into_inner()).expect("Failed storing voting poll");

        let poll_uuid = Uuid::new_v4();
        let uuid_string = poll_uuid.clone().to_string();
        let polls = active_polls.inner().clone();

        tokio::spawn(async move {
            polls.insert(uuid_string.clone(), VotingState::Started);

            // Await till switching to finished state
            sleep(Duration::from_secs(86400)).await;

            if let Some(mut v) = polls.get_mut(&uuid_string) {
                *v = VotingState::Finished;
            }

            // Await till deletion
            sleep(Duration::from_secs(2 * 86400)).await;
            polls.remove(&uuid_string);
        });

        // Implement link generation for accessing the poll (perhaps use dashmap)
        // Implement task spawning which starts a countdown
        // Implement invalidation mechanism after countdown hits 0
        Ok(poll_uuid.to_string())
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
        .manage(Arc::new(DashMap::<String, VotingState>::new()))
        .mount("/", routes![create_poll, create_user, index])
        .mount("/static", FileServer::from("static"))
}
