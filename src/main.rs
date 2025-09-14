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
        Ok(_) => Ok(()),
        Err(_) => Err(status::Custom(
            Status::InternalServerError,
            "Failed to create user by username.",
        )),
    }
}

#[post("/", data = "<body>")]
async fn create_poll(
    body: Json<VotingRequest>,
    active_polls: &State<Arc<DashMap<String, (VotingState, String)>>>,
) -> Result<String, status::Custom<&'static str>> {
    // Validate entries
    if body.validate().is_err() {
        Err(status::Custom(Status::BadRequest, "Validation failed"))
    } else {
        let poll_id = save_voting_poll(body.into_inner()).expect("Failed storing voting poll");

        let poll_uuid = Uuid::new_v4();
        let uuid_string = poll_uuid.clone().to_string();
        let polls = active_polls.inner().clone();

        tokio::spawn(async move {
            polls.insert(uuid_string.clone(), (VotingState::Started, poll_id));

            // Await till switching to finished state
            sleep(Duration::from_secs(86400)).await;

            if let Some(mut v) = polls.get_mut(&uuid_string) {
                let (state, _) = v.value_mut();
                *state = VotingState::Finished;
            }

            // Await till deletion
            sleep(Duration::from_secs(2 * 86400)).await;
            polls.remove(&uuid_string);
        });

        Ok(poll_uuid.to_string())
    }
}

#[get("/<uuid>")]
async fn get_poll(
    uuid: String,
    active_polls: &State<Arc<DashMap<String, VotingState>>>,
) -> Result<String, status::Custom<&'static str>> {
    if Uuid::parse_str(uuid.as_str()).is_err() {
        return Err(status::Custom(Status::BadRequest, "Invalid UUID format"));
    }

    if let Some(v) = active_polls.get(&uuid) {
        // get it from the database
    } else {
        return Err(status::Custom(
            Status::NotFound,
            "The provided ID does not exist",
        ));
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

    rocket::custom(figment)
        // Dashmap has Uuid as key and a state and db id tuple as value
        .manage(Arc::new(DashMap::<String, (VotingState, String)>::new()))
        .mount("/", routes![create_poll, create_user, index])
        .mount("/static", FileServer::from("static"))
}
