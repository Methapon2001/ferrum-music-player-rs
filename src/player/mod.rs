use std::{sync::mpsc::SyncSender, time::Duration};

use rodio::{OutputStream, OutputStreamBuilder, Sink, source::SeekError};

use crate::track::Track;

mod mpris;
use mpris::Mpris;

pub enum MediaPlayerEvent {
    Tick,

    PlaybackEnded,
    PlaybackProgress,
}

#[derive(Debug, Clone, Copy)]
pub enum MediaPlayerStatus {
    Stopped,
    Running,
    Paused,
}

#[allow(dead_code)]
pub struct MediaPlayer {
    player_tx: SyncSender<MediaPlayerEvent>,

    stream: OutputStream,
    sink: Sink,
    mpris: Mpris,
    status: MediaPlayerStatus,

    track: Option<Track>,
}

impl MediaPlayer {
    pub fn new(player_tx: SyncSender<MediaPlayerEvent>) -> Self {
        let stream = OutputStreamBuilder::open_default_stream().expect("Audio output stream.");
        let sink = Sink::connect_new(stream.mixer());

        let mpris = Mpris::new(player_tx.clone());

        Self {
            player_tx,

            stream,
            sink,
            mpris,

            track: None,
            status: MediaPlayerStatus::Stopped,
        }
    }

    pub fn add(&mut self, track: Track) {
        if let Ok(file) = std::fs::File::open(track.path.as_path()) {
            self.mpris.play(track.as_ref().into());
            self.track = Some(track);

            self.sink.clear();
            self.sink.append(rodio::Decoder::try_from(file).unwrap());
        }
    }

    #[inline]
    pub fn play(&mut self) {
        if self.sink.empty() {
            return;
        }

        self.status = MediaPlayerStatus::Running;
        self.sink.play();
    }

    #[inline]
    pub fn pause(&mut self) {
        if self.sink.empty() {
            return;
        }

        self.status = MediaPlayerStatus::Paused;
        self.sink.pause();
    }

    #[inline]
    pub fn stop(&mut self) {
        self.status = MediaPlayerStatus::Stopped;
        self.sink.stop();
        self.sink.clear();
    }

    #[inline]
    pub fn toggle(&mut self) {
        if self.is_paused() {
            self.play();
        } else {
            self.pause();
        }
    }

    #[inline]
    pub fn seek(&self, position: Duration) {
        match self.sink.try_seek(position) {
            Err(SeekError::NotSupported { .. }) => {
                dbg!("Seeking does not support on underlying source.");
            }
            Err(error) => {
                dbg!(error);
            }
            Ok(_) => {}
        }
    }

    #[inline]
    pub fn is_paused(&self) -> bool {
        self.sink.is_paused()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.sink.empty()
    }

    #[inline]
    pub fn set_volume(&mut self, value: f32) {
        self.mpris.set_volume(value.clamp(0.0, 1.0) as f64);
        self.sink.set_volume(value.clamp(0.0, 1.2));
    }

    #[inline]
    pub fn get_volume(&self) -> f32 {
        self.sink.volume()
    }

    #[inline]
    pub fn get_position(&self) -> Duration {
        self.sink.get_pos()
    }

    #[inline]
    pub fn get_track(&self) -> Option<&Track> {
        self.track.as_ref()
    }

    pub fn is_playing_track(&self, track: &Track) -> bool {
        if let Some(current_track) = self.get_track()
            && current_track.path == track.path
        {
            current_track.path == track.path
        } else {
            false
        }
    }
}
