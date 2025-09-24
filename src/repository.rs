use std::error::Error;
use std::io;
use std::io::ErrorKind;

use crate::models::VotingOptionResponse;
use crate::models::VotingRequest;
use crate::models::VotingResponse;
use chrono::DateTime;
use chrono::Utc;
use log::info;
use log::warn;
use rusqlite::Connection;
use rusqlite::Result;
use rusqlite::params;

fn exists_user(username: &str) -> Result<bool, rusqlite::Error> {
    let conn = Connection::open("voting_db.db3")?;

    let count = conn.query_one("SELECT 1 FROM user WHERE username = ?1", [username], |r| {
        r.get::<_, i64>(0)
    })?;

    if count == 1 { Ok(true) } else { Ok(false) }
}

pub fn save_voting_poll(poll: VotingRequest) -> Result<i64, Box<dyn Error>> {
    let conn = Connection::open("voting_db.db3")?;

    let mut stmt = conn.prepare(
        "INSERT INTO voting (state, title, voting_time_mins, username, is_multi) VALUES (?1, ?2, ?3, ?4, ?5) RETURNING id;",
    )?;

    exists_user(&poll.username)
        .map_err(|e| io::Error::new(ErrorKind::Other, format!("Username check failed {e}")))?
        .then_some(())
        .ok_or_else(|| io::Error::new(ErrorKind::NotFound, "Username does not exist"));

    let voting_id = stmt.query_row(
        (
            &poll.state.as_str(),
            poll.title,
            &poll.voting_time,
            &poll.username,
            &poll.is_multi,
        ),
        |r| r.get::<_, i64>(0),
    )?;

    for option in poll.options {
        // TODO: implement validation for is_multi
        let mut stmt = conn.prepare(
            "INSERT INTO voting_options (title, is_selected, voting_id) VALUES (?1, ?2, ?3) RETURNING id;",
        )?;

        let option_id = stmt.query_row((option.title, option.is_selected, voting_id), |r| {
            r.get::<_, i64>(0)
        })?;

        if option.is_selected {
            conn.execute(
                "INSERT INTO user_vote (username, voting_opt_id) VALUES (?1, ?2)",
                (&poll.username, option_id),
            )?;
        }
    }

    Ok(voting_id)
}

pub fn create_user(username: String) -> Result<bool, Box<dyn Error>> {
    let conn = Connection::open("voting_db.db3")?;

    if exists_user(&username)? {
        return Ok(false);
    }

    conn.execute("INSERT INTO user VALUES (?1)", [&username])?;
    Ok(true)
}

pub fn get_poll_by_id(id: &i64) -> Result<VotingResponse, rusqlite::Error> {
    let conn = Connection::open("voting_db.db3")?;
    let mut get_poll = conn.prepare(
        "SELECT v.title, v.created_at, v.voting_time_mins, v.state, v.is_multi FROM voting v WHERE v.id = ?1",
    )?;

    let mut poll = get_poll.query_row([id], |r| {
        Ok(VotingResponse {
            title: r.get(0)?,
            remaining_time: calc_remaining_time(r.get(1)?, r.get(2)?),
            options: vec![],
            state: r.get(3)?,
            is_multi: r.get(4)?,
        })
    })?;
    println!("{}", poll.title);

    let mut get_options = conn.prepare(
        "SELECT vo.id, vo.title, vo.is_selected FROM voting_options vo WHERE vo.voting_id = ?1",
    )?;

    // TODO: should return Vec<VotingOption>
    let voting_options: Vec<VotingOptionResponse> = get_options
        .query_map([id], |r| {
            Ok(VotingOptionResponse {
                id: r.get(0)?,
                title: r.get(1)?,
                is_selected: r.get(2)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    poll.options = voting_options;

    Ok(poll)
}

// Assuming timestamps are stored in the ISO 8601 format -> "2025-04-02 12:12:12"
fn calc_remaining_time(timestamp: String, voting_time: u32) -> i64 {
    let naive = chrono::NaiveDateTime::parse_from_str(&timestamp, "%Y-%m-%d %H:%M:%S")
        .expect("Failed parsing timestamp");

    let utc_stamp = DateTime::<Utc>::from_utc(naive, Utc);
    voting_time as i64 - Utc::now().signed_duration_since(utc_stamp).num_minutes()
}

pub fn update_vote(
    poll_id: i64,
    option_ids: Vec<i64>,
    username: String,
) -> Result<i64, rusqlite::Error> {
    // voting_opt has to be part of poll
    // because multiple votes are possible, a deletesert is implemented

    let conn = Connection::open("voting_db.db3")?;

    // check if provided selected options are part of poll
    for o in &option_ids {
        let mut exists_opt = conn.prepare(
            "SELECT EXISTS(SELECT 1 FROM voting_options vo WHERE vo.voting_id = ?1 AND vo.id = ?2);",
        )?;

        if !exists_opt.query_row(params![&poll_id, &o], |r| r.get(0))? {
            warn!("User {username} has provided a non-existent option id ({o}) to update the poll");
            return Err(rusqlite::Error::InvalidParameterName(
                "Nonexistent poll option".into(),
            ));
        }
    }

    // Delete the entry
    conn.execute(
        "DELETE FROM user_vote
        WHERE username = ?1
          AND voting_opt_id IN (
            SELECT vo.id
            FROM voting_options vo
            WHERE vo.voting_id = ?2
          );",
        params![&username, &poll_id],
    )?;

    for o in &option_ids {
        conn.execute(
            "INSERT INTO user_vote (username, voting_opt_id) VALUES (?1, ?2)",
            params![&username, o],
        )?;
    }

    info!("user {username} has deleted updated poll {poll_id} deleting");

    Ok(poll_id)
}

pub fn init_db() -> Result<(), Box<dyn Error>> {
    let conn = Connection::open("voting_db.db3")?;

    conn.execute("DROP TABLE IF EXISTS user_vote", ())?;
    conn.execute("DROP TABLE IF EXISTS voting_options", ())?;
    conn.execute("DROP TABLE IF EXISTS user", ())?;
    conn.execute("DROP TABLE IF EXISTS voting", ())?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS user(
            username TEXT PRIMARY KEY
        ) STRICT;",
        (),
    )?;

    conn.execute(
        "CREATE TABLE voting(
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL,
            state TEXT NOT NULL,
            created_at TEXT DEFAULT CURRENT_TIMESTAMP,
            voting_time_mins INTEGER NOT NULL,
            username TEXT NOT NULL,
            is_multi INTEGER NOT NULL,
            FOREIGN KEY (username) REFERENCES user(username)
        ) STRICT;",
        (),
    )?;

    // TODO: Voting option identifiers should be UUIDs
    conn.execute(
        "CREATE TABLE voting_options(
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL,
            is_selected INTEGER NOT NULL,
            voting_id INTEGER NOT NULL,
            FOREIGN KEY (voting_id) REFERENCES voting(id)
        ) STRICT;",
        (),
    )?;

    conn.execute(
        "CREATE TABLE user_vote(
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
