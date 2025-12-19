use rand::{Rng as _, seq::SliceRandom as _};

use crate::track::Track;

pub enum PlaylistMode {
    NoRepeat,
    Repeat,
    RepeatSingle,
    Shuffle,
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
            mode: PlaylistMode::Repeat,
            tracks,

            current_index: 0,
            previous_index: Vec::new(),
        }
    }

    pub fn select_track(&mut self, index: usize) {
        self.previous_index = Vec::new();
        self.current_index = index;
    }

    pub fn current_track(&self) -> Option<&Track> {
        self.tracks.get(self.current_index)
    }

    pub fn next_track(&mut self) -> Option<&Track> {
        match self.mode {
            PlaylistMode::Repeat => {
                self.current_index += 1;
                self.current_track()
            }
            PlaylistMode::RepeatSingle => self.current_track(),
            PlaylistMode::Shuffle => {
                let mut rng = rand::rng();
                self.previous_index.push(self.current_index);
                self.current_index = rng.random_range(0..self.tracks.len());
                self.current_track()
            }
            PlaylistMode::NoRepeat => {
                self.previous_index = Vec::new();
                None
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

    pub fn shuffle(&mut self) {
        self.tracks.shuffle(&mut rand::rng());
    }

    pub fn clear(&mut self) {
        self.tracks = Vec::new();
        self.previous_index = Vec::new();
    }

    pub fn append(&mut self, track: Track) {
        self.tracks.push(track);
    }

    pub fn append_multiple(&mut self, mut tracks: Vec<Track>) {
        self.tracks.append(&mut tracks);
    }
}
