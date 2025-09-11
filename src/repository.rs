use std::error::Error;

use crate::models::VotingRequest;
use rusqlite::Connection;

pub fn save_voting_poll(poll: VotingRequest) -> Result<(), Box<dyn Error>> {
    let conn = Connection::open("voting_db.db3")?;

    conn.execute(
        "INSERT INTO voting VALUES ?1, ?2, ?3",
        (&poll.state.as_str(), &poll.voting_time, &poll.username),
    )?;

    let voting_id: i64 = conn.query_row("SELECT last_insert_rowid()", [], |row| row.get(0))?;

    for option in poll.options {
        conn.execute(
            "INSERT INTO voting_options VALUES ?1, ?2",
            (option.title, &voting_id),
        )?;

        if option.is_selected {
            let selected_option_id: i64 =
                conn.query_row("SELECT last_insert_rowid()", [], |row| row.get(0))?;

            conn.execute(
                "INSERT INTO user_vote VALUES ?1, ?2",
                (&poll.username, selected_option_id),
            )?;
        }
    }

    Ok(())
}

pub fn create_user(username: String) -> Result<(), Box<dyn Error>> {
    let conn = Connection::open("voting_db.db3")?;

    conn.execute("INSERT INTO user VALUES (?1)", [&username])?;
    Ok(())
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
            voting_id INTEGER NOT NULL,
            FOREIGN KEY (voting_id) REFERENCES voting(id),
            FOREIGN KEY (username) REFERENCES user(username)
        ) STRICT;",
        (),
    )?;

    Ok(())
}
