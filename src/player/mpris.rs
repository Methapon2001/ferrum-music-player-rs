use std::sync::mpsc::{self, Receiver};

use souvlaki::{
    MediaControlEvent, MediaControls, MediaMetadata, MediaPlayback, MediaPosition, PlatformConfig,
};

use crate::player::{MediaPlayer, MediaPlayerStatus};

pub(super) struct Mpris {
    controls: MediaControls,
    controls_rx: Receiver<MediaControlEvent>,
}

impl Mpris {
    pub fn new<T>(handle_submit: Option<T>) -> Self
    where
        T: Fn() + Send + 'static,
    {
        let mut controls = MediaControls::new(PlatformConfig {
            dbus_name: "ferrum_player_rs",
            display_name: "Ferrum Player",
            hwnd: None,
        })
        .unwrap();

        let (controls_tx, controls_rx) = mpsc::sync_channel(32);

        controls
            .attach(move |event| {
                controls_tx.send(event.to_owned()).ok();

                if let Some(handle) = &handle_submit {
                    handle();
                }
            })
            .ok();

        Self {
            controls,
            controls_rx,
        }
    }

    pub fn try_recv_event(&mut self) -> Option<MediaControlEvent> {
        self.controls_rx.try_recv().ok()
    }

    pub fn play(&mut self, metadata: MediaMetadata) {
        self.controls.set_metadata(metadata).ok();
    }

    pub fn update_progress(&mut self, state: MediaPlayback) {
        self.controls.set_playback(state).ok();
    }

    pub fn set_volume(&mut self, volume: f64) {
        self.controls.set_volume(volume).ok();
    }
}

impl MediaPlayer {
    pub fn mpris_event(&mut self) -> Option<MediaControlEvent> {
        self.mpris.try_recv_event()
    }

    pub fn mpris_handle(&mut self, event: MediaControlEvent) {
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

        if self.is_empty() {
            self.status = MediaPlayerStatus::Stopped;
        }
    }

    pub fn mpris_update_progress(&mut self) {
        self.mpris.update_progress(match self.status {
            MediaPlayerStatus::Running => MediaPlayback::Playing {
                progress: Some(MediaPosition(self.get_position())),
            },
            MediaPlayerStatus::Paused => MediaPlayback::Paused {
                progress: Some(MediaPosition(self.get_position())),
            },
            MediaPlayerStatus::Stopped => MediaPlayback::Stopped,
        });
    }
}
