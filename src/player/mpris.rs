use std::sync::mpsc::{self, Receiver, Sender};

use log::info;
use souvlaki::{
    MediaControlEvent, MediaControls, MediaMetadata, MediaPlayback, MediaPosition, PlatformConfig,
};

use crate::player::{GeneralMusicPlayer as _, MusicPlayer, MusicPlayerEvent, MusicPlayerStatus};

pub(super) struct Mpris {
    controls: MediaControls,
    controls_rx: Receiver<MediaControlEvent>,
}

impl Mpris {
    pub fn new(player_tx: Sender<MusicPlayerEvent>) -> Self {
        let mut controls = MediaControls::new(PlatformConfig {
            dbus_name: "org.ferrum.Player",
            display_name: "Ferrum Player",
            hwnd: None,
        })
        .expect("Media controls");

        let (controls_tx, controls_rx) = mpsc::sync_channel(32);

        controls
            .attach(move |event| {
                controls_tx.send(event).ok();
                player_tx.send(MusicPlayerEvent::Tick).ok();
            })
            .ok();

        Self {
            controls,
            controls_rx,
        }
    }

    pub fn try_recv_event(&self) -> Option<MediaControlEvent> {
        self.controls_rx.try_recv().ok()
    }

    pub fn set_metadata(&mut self, metadata: MediaMetadata<'_>) {
        self.controls.set_metadata(metadata).ok();
    }

    pub fn set_volume(&mut self, volume: f64) {
        self.controls.set_volume(volume).ok();
    }

    pub fn update_progress(&mut self, state: MediaPlayback) {
        self.controls.set_playback(state).ok();
    }
}

impl MusicPlayer {
    pub fn mpris_event(&self) -> Option<MediaControlEvent> {
        self.mpris.try_recv_event()
    }

    pub fn mpris_handle(&mut self, event: &MediaControlEvent) {
        match event {
            MediaControlEvent::SetVolume(value) => self.set_volume(*value as f32),
            MediaControlEvent::Play => self.play(),
            MediaControlEvent::Next => self.play_next(),
            MediaControlEvent::Previous => self.play_previous(),
            MediaControlEvent::Pause => self.pause(),
            MediaControlEvent::Toggle => self.toggle(),
            MediaControlEvent::Stop => self.stop(),
            MediaControlEvent::SetPosition(MediaPosition(value)) => self.seek(*value),
            _ => {
                info!("MPRIS event received but not implemented.");
            }
        }

        if self.is_stopped() {
            self.status = MusicPlayerStatus::Stopped;
        }
    }

    pub fn mpris_update_progress(&mut self) {
        self.mpris.update_progress(match self.status {
            MusicPlayerStatus::Playing => MediaPlayback::Playing {
                progress: Some(MediaPosition(self.position())),
            },
            MusicPlayerStatus::Paused => MediaPlayback::Paused {
                progress: Some(MediaPosition(self.position())),
            },
            MusicPlayerStatus::Stopped => MediaPlayback::Stopped,
        });
    }
}
