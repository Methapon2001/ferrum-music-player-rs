use std::{path::PathBuf, sync::Arc, time::Duration};

use eframe::egui::mutex::{Mutex, MutexGuard};
use rusqlite::{Connection, named_params};

use crate::{config::get_default_app_dir_config, track::Track};

#[derive(Clone)]
pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

impl Database {
    fn migrate(conn: &Connection) -> Result<(), rusqlite::Error> {
        conn.execute_batch(include_str!("./migrations/001.sql"))?;

        Ok(())
    }

    pub fn new() -> Result<Self, rusqlite::Error> {
        let conn = Connection::open(get_default_app_dir_config().join("library.db"))?;

        Self::migrate(&conn).ok();

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn get_connection(&self) -> MutexGuard<'_, Connection> {
        self.conn.lock()
    }
}

pub fn get_all_tracks(conn: &Connection) -> Result<Vec<Track>, rusqlite::Error> {
    let mut stmt = conn.prepare_cached(include_str!("./sql/get_all_tracks.sql"))?;

    stmt.query_map(named_params! {}, |row| {
        Ok(Track {
            path: row.get("path").map(|v: String| PathBuf::from(v))?,
            title: row.get("title").ok(),
            artist: row.get("artist").ok(),
            genre: row.get("genre").ok(),
            album: row.get("album").ok(),
            album_artist: row.get("album_artist").ok(),
            track: row.get("track").ok(),
            track_total: row.get("track_total").ok(),
            disc: row.get("disc").ok(),
            disc_total: row.get("disc_total").ok(),
            duration: row
                .get("duration")
                .map(|v: i32| Duration::from_secs(u64::try_from(v.max(0)).unwrap()))
                .ok(),
            ..Default::default()
        })
    })?
    .collect()
}

pub fn upsert_track(conn: &Connection, track: &Track) -> Result<i32, rusqlite::Error> {
    let mut stmt = conn.prepare_cached(include_str!("./sql/upsert_track.sql"))?;

    stmt.query_row(
        named_params! {
            ":path": track.path.to_string_lossy(),
            ":title": track.title,
            ":artist": track.artist,
            ":genre": track.genre,
            ":album": track.album,
            ":album_artist": track.album_artist,
            ":track": track.track,
            ":track_total": track.track_total,
            ":disc": track.disc,
            ":disc_total": track.disc_total,
            ":duration": track.duration.map(|v| v.as_secs()),
        },
        |row| row.get(0),
    )
}
