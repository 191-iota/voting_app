use rocket::serde::Deserialize;
use rocket::tokio::sync::broadcast;
use serde::Serialize;
use validator::Validate;
use validator::ValidationError;

// TODO: Rename Poll  -> Poll

pub struct PollSession {
    pub tx: broadcast::channel::<VoteUpdate>(16),
    pub state: PollState,
    pub db_id: i64,
}

impl PollSession {
    pub fn new(db_id: i64) -> Self {
        let (tx, _) = broadcast::channel::<VoteUpdate>(16);
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
    // TODO: Implement rule that one option has to be selected
    #[validate(length(min = 1), custom(function = "validate_min_selection"))]
    pub options: Vec<PollOptionRequest>,
    pub state: PollState,
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

#[derive(Serialize, Deserialize)]
pub struct PollUpdateRequest {
    pub username: String,
    pub poll_id: String,
    pub voted_option_uuids: Vec<String>,
}

#[derive(Deserialize, Validate)]
pub struct PollOptionRequest {
    #[validate(length(min = 1, max = 255))]
    pub title: String,
    pub is_selected: bool,
}

#[derive(Serialize)]
pub struct PollOptionResponse {
    pub id: i64,
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

fn validate_min_selection(options: Vec<PollOptionRequest>) -> Result<(), ValidationError> {
    Ok(())
}
