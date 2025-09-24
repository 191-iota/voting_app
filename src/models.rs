use rocket::serde::Deserialize;
use rocket::tokio::sync::broadcast;
use serde::Serialize;
use std::collections::HashMap;
use validator::Validate;
use validator::ValidationError;

// TODO: Rename Voting  -> Poll

pub struct PollSession {
    pub tx: broadcast::Sender<i64>,
    pub state: VotingState,
    pub db_id: i64,
}

impl PollSession {
    pub fn new(db_id: i64) -> Self {
        let (tx, _) = broadcast::channel(16);
        Self {
            tx,
            state: VotingState::Started,
            db_id,
        }
    }
}

pub struct VotingSession {
    pub title: String,
    pub remaining_time: u32,
    pub options: Vec<HashMap<String, u32>>,
}

#[derive(Deserialize, Validate)]
pub struct VotingRequest {
    #[validate(length(min = 3, max = 50))]
    pub username: String,
    #[validate(length(min = 3, max = 50))]
    pub title: String,
    #[validate(range(min = 1, max = 255))]
    pub voting_time: u32,
    // TODO: Implement rule that one option has to be selected
    #[validate(length(min = 1), custom(function = "validate_min_selection"))]
    pub options: Vec<VotingOptionRequest>,
    pub state: VotingState,
    pub is_multi: bool,
}

#[derive(Serialize)]
pub struct VotingResponse {
    pub title: String,
    pub remaining_time: i64,
    pub options: Vec<VotingOptionResponse>,
    pub state: String,
    pub is_multi: bool,
}

#[derive(Serialize, Deserialize)]
pub struct VotingUpdateRequest {
    pub username: String,
    pub poll_id: String,
    pub voted_option_ids: Vec<i64>,
}

#[derive(Deserialize, Validate)]
pub struct VotingOptionRequest {
    #[validate(length(min = 1, max = 255))]
    pub title: String,
    pub is_selected: bool,
}

#[derive(Serialize)]
pub struct VotingOptionResponse {
    pub id: i64,
    pub title: String,
    pub is_selected: bool,
}

#[derive(Deserialize, PartialEq, Eq)]
pub enum VotingState {
    Started,
    Finished,
}

impl VotingState {
    pub fn as_str(&self) -> &'static str {
        match self {
            VotingState::Started => "Started",
            VotingState::Finished => "Finished",
        }
    }
}

fn validate_min_selection(options: Vec<VotingOptionRequest>) -> Result<(), ValidationError> {
    Ok(())
}
