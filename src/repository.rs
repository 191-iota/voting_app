use std::error::Error;

use crate::models::VotingRequest;
use crate::models::VotingResponse;
use chrono::DateTime;
use chrono::Utc;
use rusqlite::{Connection, Result};

pub fn save_voting_poll(poll: VotingRequest) -> Result<String, Box<dyn Error>> {
    let conn = Connection::open("voting_db.db3")?;

    let mut stmt = conn.prepare(
        "INSERT INTO voting (state, voting_time_mins, username) VALUES (?1, ?2, ?3) RETURNING id;",
    )?;

    let voting_id = stmt.query_row(
        (&poll.state.as_str(), &poll.voting_time, &poll.username),
        |r| r.get::<_, i64>(0),
    )?;

    for option in poll.options {
        let mut stmt = conn.prepare(
            "INSERT INTO voting_options (title, is_selected) VALUES (?1, ?2) RETURNING id;",
        )?;

        let option_id = stmt.query_row((option.title, &voting_id), |r| r.get::<_, i64>(0))?;

        if option.is_selected {
            conn.execute(
                "INSERT INTO user_vote (username, voting_opt_id) VALUES (?1, ?2)",
                (&poll.username, option_id),
            )?;
        }
    }

    Ok(voting_id.to_string())
}

pub fn create_user(username: String) -> Result<(), Box<dyn Error>> {
    let conn = Connection::open("voting_db.db3")?;

    conn.execute("INSERT INTO user VALUES (?1)", [&username])?;
    Ok(())
}

pub fn get_poll_by_id(id: String) -> Result<(), rusqlite::Error> {
    let conn = Connection::open("voting_db.db3")?;

    let mut sql = conn.prepare("SELECT title, created_at, voting_time_mins, state FROM voting v INNER JOIN voting_options vc on v.id = vc.voting_id WHERE v.id = ?1")?;
    let voting = sql.query_row([id], |r| {
        Ok(VotingResponse {
            title: r.get(0)?,
            remaining_time: calc_remaining_time(r.get(1)?, r.get(2)?),
            options: vec![],
            state: r.get(3)?,
        })
    })?;

    Ok(())

    // TODO: add the options of vote into hte created votingresponse object
}

// Assuming timestamps are stored in the ISO 8601 format -> "2025-04-02 12:12:12"
fn calc_remaining_time(timestamp: String, voting_time: u32) -> i64 {
    let naive = chrono::NaiveDateTime::parse_from_str(&timestamp, "%Y-%m-%d %H:%M:%S")
        .expect("Failed parsing timestamp");

    let utc_stamp = DateTime::<Utc>::from_utc(naive, Utc);
    voting_time as i64 - Utc::now().signed_duration_since(utc_stamp).num_minutes()
}

pub fn init_db() -> Result<(), Box<dyn Error>> {
    let conn = Connection::open("voting_db.db3")?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS user(
            username TEXT PRIMARY KEY
        ) STRICT;",
        (),
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS voting(
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            state TEXT NOT NULL,
            created_at TEXT DEFAULT CURRENT_TIMESTAMP,
            voting_time_mins INTEGER NOT NULL,
            username TEXT NOT NULL,
            FOREIGN KEY (username) REFERENCES user(username)
        ) STRICT;",
        (),
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS voting_options(
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL,
            voting_id INTEGER NOT NULL,
            FOREIGN KEY (voting_id) REFERENCES voting(id)
        ) STRICT;",
        (),
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS user_vote(
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            username TEXT NOT NULL,
            voting_opt_id INTEGER NOT NULL,
            FOREIGN KEY (voting_opt_id) REFERENCES voting_options(id),
            FOREIGN KEY (username) REFERENCES user(username)
        ) STRICT;",
        (),
    )?;

    Ok(())
}
