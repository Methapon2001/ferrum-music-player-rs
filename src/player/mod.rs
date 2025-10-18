use std::{sync::mpsc::Sender, time::Duration};

use rodio::{OutputStream, OutputStreamBuilder};
use souvlaki::MediaMetadata;

use crate::track::Track;

mod mpris;
use mpris::Mpris;

mod sink;
use sink::Sink;

mod source;

pub enum MusicPlayerEvent {
    Tick,

    PlaybackStarted,
    PlaybackProgress,
    PlaybackStopped,
    PlaybackEnded,
}

#[derive(Debug, Clone, Copy)]
pub enum MusicPlayerStatus {
    Stopped,
    Playing,
    Paused,
}

pub struct MusicPlayer {
    player_tx: Sender<MusicPlayerEvent>,

    #[allow(dead_code)]
    stream: OutputStream,
    sink: Sink,
    mpris: Mpris,
    status: MusicPlayerStatus,

    track: Option<Track>,
}

impl MusicPlayer {
    pub fn new(player_tx: Sender<MusicPlayerEvent>) -> Self {
        let stream = OutputStreamBuilder::open_default_stream().expect("Audio output stream.");
        let sink = Sink::new(stream.mixer(), player_tx.clone());
        let mpris = Mpris::new(player_tx.clone());

        Self {
            player_tx,

            stream,
            sink,
            mpris,

            track: None,
            status: MusicPlayerStatus::Stopped,
        }
    }

    pub fn play_track(&mut self, track: Track) {
        self.sink.stop();

        if let Ok(file) = std::fs::File::open(track.path.as_path()) {
            self.mpris.set_metadata(MediaMetadata {
                album: track.as_ref().album.as_deref(),
                title: track.as_ref().title.as_deref(),
                artist: track.as_ref().artist.as_deref(),
                duration: track.as_ref().duration,
                cover_url: None,
            });
            self.sink.add(rodio::Decoder::try_from(file).unwrap());
            self.sink.play();

            self.status = MusicPlayerStatus::Playing;
            self.track = Some(track.clone());

            self.player_tx.send(MusicPlayerEvent::PlaybackStarted).ok();
        }
    }

    #[inline]
    pub fn play(&mut self) {
        if self.sink.is_empty() {
            if let Some(track) = &self.track
                && let Ok(file) = std::fs::File::open(track.path.as_path())
            {
                self.sink.add(rodio::Decoder::try_from(file).unwrap());
            }

            return;
        }

        self.sink.play();
        self.status = MusicPlayerStatus::Playing;
    }

    #[inline]
    pub fn pause(&mut self) {
        if self.sink.is_empty() {
            return;
        }

        self.sink.pause();
        self.status = MusicPlayerStatus::Paused;
    }

    #[inline]
    pub fn stop(&mut self) {
        self.sink.stop();
        self.status = MusicPlayerStatus::Stopped;
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
    pub fn seek(&mut self, position: Duration) {
        self.sink.seek(position);
        self.mpris_update_progress();
    }

    #[inline]
    pub fn is_paused(&self) -> bool {
        self.sink.is_paused()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.sink.is_empty()
    }

    #[inline]
    pub fn set_volume(&mut self, value: f32) {
        self.mpris.set_volume(value.clamp(0.0, 1.0) as f64);
        self.sink.set_volume(value.clamp(0.0, 1.2));
    }

    #[inline]
    pub fn volume(&self) -> f32 {
        self.sink.volume()
    }

    #[inline]
    pub fn position(&self) -> Duration {
        self.sink.position()
    }

    #[inline]
    pub fn current_track(&self) -> Option<&Track> {
        self.track.as_ref()
    }
}
