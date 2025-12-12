use std::fs;
use std::io::Write as _;
use std::path::PathBuf;
use std::{collections::HashMap, path::Path};

use log::warn;
use rand::Rng as _;
use rand::seq::SliceRandom as _;

use crate::{
    config::get_default_app_dir_config,
    database::{Database, get_all_tracks},
    track::{Track, read_track_metadata},
};

#[derive(Debug)]
pub enum PlaylistMode {
    NoRepeat,
    Repeat,
    RepeatSingle,
    Random,
}

#[derive(Debug)]
pub struct Playlist {
    mode: PlaylistMode,
    tracks: Vec<Track>,

    id: Option<String>,
    current_index: usize,
    previous_index: Vec<usize>,
}

impl Default for Playlist {
    fn default() -> Self {
        Self {
            mode: PlaylistMode::Repeat,
            tracks: Vec::new(),

            id: None,
            current_index: 0,
            previous_index: Vec::new(),
        }
    }
}

impl Playlist {
    /// Create playlist from given tracks without id.
    ///
    /// # Returns
    ///
    /// A `Playlist` with:
    /// * `Id` - Default to `None` which is intended to be used as default playback playlist.
    pub fn new(tracks: Vec<Track>) -> Self {
        Self {
            mode: PlaylistMode::Repeat,
            tracks,

            id: None,
            current_index: 0,
            previous_index: Vec::new(),
        }
    }

    /// Read and create from file.
    ///
    /// This function will create playlist with track metadata from library.
    /// If track metadata does not exists in the library fallback to read from file.
    ///
    /// # Returns
    ///
    /// A `Playlist` with:
    /// * `Id` - Default to file name.
    pub fn new_from_file(path: &Path) -> std::io::Result<Self> {
        // NOTE: Try to get library metadata from the database so that we can get track metadata
        // from library instead of trying to read from the file directly.
        let tracks: HashMap<PathBuf, Track> = match Database::new() {
            Ok(database) => get_all_tracks(&database.get_connection())
                .map(|tracks| {
                    tracks
                        .into_iter()
                        .map(|track| (track.path.clone(), track))
                        .collect()
                })
                .unwrap_or_default(),
            Err(_) => Default::default(),
        };

        let content = fs::read_to_string(path)?;

        let mut playlist_tracks = Vec::new();

        for line in content.lines() {
            let path = Path::new(line);

            if let Some(track) = tracks.get(path) {
                playlist_tracks.push(track.to_owned());
            } else {
                match read_track_metadata(path) {
                    Ok(metadata) => playlist_tracks.push(metadata),
                    Err(err) => {
                        warn!(
                            "Unable to read track metadata from '{:?}' - {err:?}",
                            path.display(),
                        );

                        playlist_tracks.push(Track {
                            path: path.to_owned(),
                            ..Default::default()
                        });
                    }
                }
            }
        }

        let mut playlist = Self::new(playlist_tracks);

        if let Some(name) = path.file_name() {
            playlist.id(name.to_string_lossy().to_string());
        }

        Ok(playlist)
    }

    pub fn id(&mut self, id: String) {
        self.id = Some(id);
    }

    pub fn get_id(&self) -> Option<&str> {
        self.id.as_deref()
    }

    pub fn select_track(&mut self, index: usize) {
        self.previous_index = Vec::new();
        self.current_index = index.clamp(0, self.tracks.len() - 1);
    }

    pub fn current_track_index(&self) -> usize {
        self.current_index
    }

    pub fn current_track(&self) -> Option<&Track> {
        self.tracks.get(self.current_index)
    }

    pub fn next_track(&mut self) -> Option<&Track> {
        match self.mode {
            PlaylistMode::NoRepeat => {
                self.previous_index = Vec::new();
                None
            }
            PlaylistMode::Repeat => {
                self.current_index += 1;

                if self.current_index >= self.tracks.len() {
                    self.current_index = 0;
                }

                self.current_track()
            }
            PlaylistMode::RepeatSingle => self.current_track(),
            PlaylistMode::Random => {
                let mut rng = rand::rng();
                self.previous_index.push(self.current_index);
                self.current_index = rng.random_range(0..self.tracks.len());
                self.current_track()
            }
        }
    }

    pub fn previous_track(&mut self) -> Option<&Track> {
        if let Some(previous_index) = self.previous_index.pop() {
            self.current_index = previous_index;
        } else {
            self.current_index = self.current_index.saturating_sub(1);
        }

        self.current_track()
    }

    pub fn tracks(&self) -> &[Track] {
        self.tracks.as_slice()
    }

    pub fn set_mode(&mut self, mode: PlaylistMode) {
        self.mode = mode;
    }

    pub fn mode(&self) -> &PlaylistMode {
        &self.mode
    }

    pub fn position(&self) -> usize {
        self.current_index
    }

    pub fn shuffle(&mut self) {
        self.tracks.shuffle(&mut rand::rng());
    }

    pub fn clear(&mut self) {
        self.tracks = Vec::new();
        self.previous_index = Vec::new();
    }

    pub fn push(&mut self, track: Track) {
        self.tracks.push(track);
    }

    pub fn append(&mut self, mut tracks: Vec<Track>) {
        self.tracks.append(&mut tracks);
    }

    pub fn save(&self) {
        let mut content = String::new();

        self.tracks.iter().for_each(|track| {
            if let Some(path) = track.path.to_str() {
                content.push_str(path);
                content.push('\n');
            }
        });

        let file_path = get_default_app_dir_config().join("playlist.m3u");

        if let Ok(file) = &mut fs::File::create(file_path) {
            file.write_all(content.into_bytes().as_slice()).ok();
        }
    }
}

impl PartialEq for Playlist {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use crate::database::{Database, get_all_tracks};

    #[test]
    fn read_playlist_from_file() {
        Playlist::new_from_file(&get_default_app_dir_config().join("playlist.m3u")).ok();
    }

    #[test]
    fn write_playlist_to_file() {
        let database = Database::new().expect("Database connected.");
        let playlist =
            Playlist::new(get_all_tracks(&database.get_connection()).unwrap_or_default());

        playlist.save();
    }
}
