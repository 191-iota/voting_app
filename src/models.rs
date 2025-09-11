use rocket::serde::Deserialize;
use std::collections::HashMap;
use validator::Validate;

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
    pub options: Vec<VotingOption>,
    pub state: VotingState,
}

#[derive(Deserialize, Validate)]
pub struct VotingOption {
    #[validate(length(min = 1, max = 255))]
    pub title: String,
    pub is_selected: bool,
}

#[derive(Deserialize)]
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
