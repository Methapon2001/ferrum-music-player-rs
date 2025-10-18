use rand::{Rng, seq::SliceRandom};

use crate::track::Track;

#[derive(Default)]
pub enum PlaylistMode {
    #[default]
    Playlist,
    Random,
    Single,
}

pub struct Playlist {
    mode: PlaylistMode,
    tracks: Vec<Track>,

    current_index: usize,
    previous_index: Vec<usize>,
}

impl Playlist {
    pub fn new(tracks: Vec<Track>) -> Self {
        Self {
            mode: PlaylistMode::Playlist,
            tracks,

            current_index: 0,
            previous_index: Vec::new(),
        }
    }

    pub fn set_current_track_index(&mut self, index: usize) {
        self.current_index = index;
    }

    pub fn get_current_track_index(&self) -> usize {
        self.current_index
    }

    pub fn current_track(&self) -> Option<&Track> {
        self.tracks.get(self.current_index)
    }

    pub fn next_track(&mut self) -> Option<&Track> {
        match self.mode {
            PlaylistMode::Playlist => {
                self.previous_index.push(self.current_index);
                self.current_index += 1;
                self.current_track()
            }
            PlaylistMode::Random => {
                let mut rng = rand::rng();
                self.previous_index.push(self.current_index);
                self.current_index = rng.random_range(0..self.tracks.len());
                self.current_track()
            }
            PlaylistMode::Single => self.current_track(),
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

    pub fn shuffle(&mut self) {
        self.tracks.shuffle(&mut rand::rng());
    }

    pub fn clear(&mut self) {
        self.tracks = Vec::new();
    }

    pub fn append(&mut self, track: Track) {
        self.tracks.push(track);
    }
}
