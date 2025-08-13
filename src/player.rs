use std::{
    sync::mpsc::{self, Receiver, SyncSender},
    time::Duration,
};

use souvlaki::{
    MediaControlEvent, MediaControls, MediaMetadata, MediaPlayback, MediaPosition, PlatformConfig,
};

use crate::track::Track;

pub enum MediaPlayerEvent {
    Tick,
}

#[derive(Debug, Clone, Copy)]
pub enum MediaPlayerStatus {
    Stopped,
    Running,
    Paused,
}

#[allow(dead_code)]
pub struct MediaPlayer {
    status: MediaPlayerStatus,
    controls: MediaControls,
    controls_rx: Receiver<MediaControlEvent>,
    stream: rodio::OutputStream,
    sink: rodio::Sink,
    track: Option<Track>,
}

impl MediaPlayer {
    pub fn new(player_tx: SyncSender<MediaPlayerEvent>) -> Self {
        let stream = rodio::OutputStreamBuilder::open_default_stream().unwrap();
        let sink = rodio::Sink::connect_new(stream.mixer());

        let mut controls = MediaControls::new(PlatformConfig {
            dbus_name: "music_player_rs",
            display_name: "Music Player Rust",
            hwnd: None,
        })
        .unwrap();

        let (controls_tx, controls_rx) = mpsc::sync_channel(32);

        controls
            .attach(move |event| {
                controls_tx.send(event).ok();

                player_tx.send(MediaPlayerEvent::Tick).ok();
            })
            .ok();

        Self {
            status: MediaPlayerStatus::Stopped,
            controls,
            controls_rx,
            stream,
            sink,
            track: None,
        }
    }

    pub fn mpris_handler(&mut self) {
        if let Ok(event) = self.controls_rx.try_recv() {
            dbg!(&event);

            match event {
                MediaControlEvent::SetVolume(value) => self.set_volume(value as f32),
                MediaControlEvent::Play => self.play(),
                MediaControlEvent::Pause => self.pause(),
                MediaControlEvent::Toggle => self.toggle(),
                MediaControlEvent::Stop => self.stop(),
                MediaControlEvent::SetPosition(MediaPosition(value)) => self.seek(value),
                _ => {
                    dbg!("MPRIS event received but not implemented.");
                }
            }
        }

        if self.is_empty() {
            self.status = MediaPlayerStatus::Stopped;
        }

        self.mpris_update_progress();
    }

    pub fn mpris_update_progress(&mut self) {
        self.controls
            .set_playback(match self.status {
                MediaPlayerStatus::Running => MediaPlayback::Playing {
                    progress: Some(MediaPosition(self.get_position())),
                },
                MediaPlayerStatus::Paused => MediaPlayback::Paused {
                    progress: Some(MediaPosition(self.get_position())),
                },
                MediaPlayerStatus::Stopped => MediaPlayback::Stopped,
            })
            .ok();
    }

    pub fn add(&mut self, track: Track) {
        self.controls
            .set_metadata(MediaMetadata {
                album: track.album.as_deref(),
                title: track.title.as_deref(),
                artist: track.artist.as_deref(),
                duration: track.duration,
                cover_url: None,
            })
            .unwrap();

        if let Ok(file) = std::fs::File::open(track.path.as_path()) {
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
        self.sink.try_seek(position).unwrap()
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
    pub fn set_volume(&self, value: f32) {
        self.sink.set_volume(value.clamp(0.0, 1.2));
    }

    #[inline]
    pub fn get_position(&self) -> Duration {
        self.sink.get_pos()
    }

    pub fn get_track(&self) -> Option<&Track> {
        self.track.as_ref()
    }
}
