use rocket::serde::Deserialize;
use rocket::tokio::sync::broadcast;
use serde::Serialize;
use validator::Validate;
use validator::ValidationError;

pub struct PollSession {
    pub tx: broadcast::Sender<Vec<VoteUpdate>>,
    pub state: PollState,
    pub db_id: i64,
}

impl PollSession {
    pub fn new(db_id: i64) -> Self {
        let (tx, _) = broadcast::channel::<Vec<VoteUpdate>>(16);
        Self {
            tx,
            state: PollState::Started,
            db_id,
        }
    }
}

#[derive(Deserialize, Validate)]
pub struct PollRequest {
    #[validate(length(min = 3, max = 50))]
    pub username: String,
    #[validate(length(min = 3, max = 50))]
    pub title: String,
    #[validate(range(min = 1, max = 255))]
    pub voting_time: u32,
    #[validate(length(min = 1), custom(function = "validate_min_selection"))]
    pub options: Vec<PollOptionRequest>,
    pub is_multi: bool,
}

#[derive(Serialize, Clone)]
pub struct VoteUpdate {
    pub option_uuid: String,
    pub votes: u32,
}

#[derive(Serialize)]
pub struct PollResponse {
    pub title: String,
    pub remaining_time: i64,
    pub options: Vec<PollOptionResponse>,
    pub state: String,
    pub is_multi: bool,
}

#[derive(Serialize, Validate, Deserialize)]
pub struct PollUpdateRequest {
    pub username: String,
    pub poll_id: String,
    #[validate(length(min = 1))]
    pub selected_options: Vec<String>,
}

#[derive(Deserialize, Serialize, Validate)]
pub struct PollOptionRequest {
    #[validate(length(min = 1, max = 255))]
    pub title: String,
    pub is_selected: bool,
}

#[derive(Serialize)]
pub struct PollOptionResponse {
    pub id: String,
    pub title: String,
    pub is_selected: bool,
}

#[derive(Deserialize, PartialEq, Eq)]
pub enum PollState {
    Started,
    Finished,
}

impl PollState {
    pub fn as_str(&self) -> &'static str {
        match self {
            PollState::Started => "Started",
            PollState::Finished => "Finished",
        }
    }
}

fn validate_min_selection(options: &Vec<PollOptionRequest>) -> Result<(), ValidationError> {
    for o in options {
        if o.is_selected {
            return Ok(());
        }
    }

    Err(ValidationError::new("Must select at least one option"))
}
