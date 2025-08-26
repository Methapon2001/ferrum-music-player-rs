use std::{
    cmp::Ordering,
    collections::HashMap,
    path::PathBuf,
    str::FromStr,
    sync::Arc,
    time::{Duration, SystemTime},
};

use chrono::DateTime;
use eframe::egui::mutex::{Mutex, MutexGuard};
use rusqlite::{Connection, named_params};

use crate::{
    config::{get_default_app_dir_config, get_default_audio_dir_config},
    track::{Track, read_track_metadata, scan_tracks},
};

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

    /// This function updates a music library database based on local audio files.
    /// It either performs a full scan of all files or a partial, incremental update that only processes new or modified files.
    ///
    /// # Arguments
    ///
    /// * `full` - A boolean flag.
    ///   - If true, the function will perform a full refresh, scanning all audio files in the configured directory.
    ///   - If false, it will perform an incremental refresh, only processing files that are new or have been modified since their last entry in the database.
    pub fn refresh_library(&self, full: bool) -> Result<(), rusqlite::Error> {
        let mut conn = self.get_connection();

        let track_records: HashMap<PathBuf, Track> = get_all_tracks(&conn)
            .unwrap_or_default()
            .into_iter()
            .map(|item| (item.path.to_owned(), item))
            .collect();
        let mut track_entries = get_default_audio_dir_config()
            .as_deref()
            .map(scan_tracks)
            .unwrap_or_default();

        if !full {
            track_entries.retain(|entry| {
                track_records.get(entry).is_none_or(|v| {
                    v.modified.as_deref().is_none_or(|modified| {
                        let record_modified_dt =
                            DateTime::<chrono::Local>::from_str(modified).unwrap();
                        let source_modified_dt = DateTime::<chrono::Local>::from(
                            entry
                                .metadata()
                                .and_then(|metadata| metadata.modified())
                                .unwrap_or(SystemTime::now()),
                        );

                        source_modified_dt.cmp(&record_modified_dt) == Ordering::Greater
                    })
                })
            });
        }

        if let Ok(tx) = conn.transaction() {
            track_entries.iter().for_each(|entry| {
                if let Err(err) =
                    upsert_track(&tx, &read_track_metadata(entry).expect("Music metadata."))
                {
                    dbg!("Failed to update database:", err);
                };
            });

            tx.commit()?;
        }

        Ok(())
    }
}

pub fn get_all_tracks(conn: &Connection) -> Result<Vec<Track>, rusqlite::Error> {
    let mut stmt = conn.prepare_cached(include_str!("./sql/get_all_tracks.sql"))?;

    stmt.query_map(named_params! {}, |row| {
        Ok(Track {
            path: row.get("path").map(|v: String| PathBuf::from(v))?,
            modified: row.get("modified").ok(),
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
            ":modified": track.modified,
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
