use std::sync::Arc;

use dashmap::DashMap;
use log::warn;
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

use self::models::PollSession;
use self::models::VotingResponse;
use self::models::VotingState;
use self::models::VotingUpdateRequest;
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
        Ok(v) => {
            if v == true {
                Err(status::Custom(
                    Status::ExpectationFailed,
                    "Username already exists",
                ))
            } else {
                Ok(())
            }
        }
        Err(_) => Err(status::Custom(
            Status::InternalServerError,
            "Failed to create user by username.",
        )),
    }
}

#[post("/", data = "<body>")]
async fn create_poll(
    body: Json<VotingRequest>,
    active_polls: &State<Arc<DashMap<String, PollSession>>>,
) -> Result<String, status::Custom<&'static str>> {
    if body.validate().is_err() {
        Err(status::Custom(Status::BadRequest, "Validation failed"))
    } else {
        // TODO: Avoid using box pointer here for errors
        let result = save_voting_poll(body.into_inner()).expect("Failed saving the poll");

        let poll_uuid = Uuid::new_v4();
        let uuid_string = poll_uuid.clone().to_string();
        let polls = active_polls.inner().clone();

        tokio::spawn(async move {
            polls.insert(uuid_string.clone(), PollSession::new(result));

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
    active_polls: &State<Arc<DashMap<String, (VotingState, i64)>>>,
) -> Result<Json<VotingResponse>, status::Custom<&'static str>> {
    if Uuid::parse_str(uuid.as_str()).is_err() {
        return Err(status::Custom(Status::BadRequest, "Invalid UUID format"));
    }

    if let Some(v) = active_polls.get(&uuid) {
        let (_, v) = v.value();
        // get it from the database
        let result = repository::get_poll_by_id(v);
        match result {
            Ok(v) => Ok(Json(v)),
            Err(e) => {
                warn!("non existent id access: db poll id, error: {e}");
                Err(status::Custom(
                    Status::NotFound,
                    "The provided ID does not exist",
                ))
            }
        }
    } else {
        warn!("non existent id access: in-memory poll id");
        Err(status::Custom(
            Status::NotFound,
            "The provided ID does not exist",
        ))
    }
}

#[put("/", data = "<req>")]
async fn update_poll(
    req: Json<VotingUpdateRequest>,
    active_polls: &State<Arc<DashMap<String, (VotingState, i64)>>>,
) -> Result<String, status::Custom<&'static str>> {
    let body = req.into_inner();
    if Uuid::parse_str(&body.poll_id).is_err() {
        return Err(status::Custom(Status::BadRequest, "Invalid UUID format"));
    }

    let poll = active_polls
        .get(&body.poll_id)
        .ok_or(status::Custom(Status::NotFound, "Poll not found"))?;

    let (state, db_id) = poll.value();

    if state != &VotingState::Started {
        return Err(status::Custom(Status::BadRequest, "Poll not active"));
    }

    match repository::update_vote(*db_id, body.voted_option_ids, body.username) {
        Ok(v) => Ok(v.to_string()),
        Err(e) => {
            warn!("non existent id access: db poll id, error: {e}");
            Err(status::Custom(Status::BadRequest, "Failed updating vote"))
        }
    }
}

// TODO: implement via broadcast rx tx eventstream an endpoint which live updates the view
#[get("/<id>/live")]
async fn get_live_poll_update(
    id: String,
    active_polls: &State<Arc<DashMap<String, (VotingState, i64)>>>,
    mut end: rocket::Shutdown,
) {
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
        .unwrap_or(false);

    if do_init {
        repository::init_db().expect("Failed to initialize DB");
    }

    rocket::custom(figment)
        // Dashmap has Uuid as key and a state and db id tuple as value
        .manage(Arc::new(DashMap::<String, PollSession>::new()))
        .mount(
            "/",
            routes![create_poll, create_user, index, get_poll, update_poll],
        )
        .mount("/static", FileServer::from("static"))
}
