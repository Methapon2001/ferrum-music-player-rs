use std::sync::mpsc::Sender;
use std::time::Duration;

use rodio::{OutputStream, OutputStreamBuilder};
use souvlaki::MediaMetadata;

use crate::playlist::Playlist;
use crate::track::Track;

mod mpris;
use mpris::Mpris;

mod sink;
use sink::Sink;

mod source;

pub trait GeneralMusicPlayer {
    fn play_track(&mut self, track: &Track);

    fn play_next(&mut self);

    fn play_previous(&mut self);

    fn playlist(&self) -> &Playlist;

    fn playlist_mut(&mut self) -> &mut Playlist;

    fn play(&mut self);

    fn pause(&mut self);

    fn stop(&mut self);

    fn toggle(&mut self);

    fn seek(&mut self, position: Duration);

    fn is_paused(&self) -> bool;

    fn is_stopped(&self) -> bool;

    fn set_volume(&mut self, value: f32);

    fn volume(&self) -> f32;

    fn position(&self) -> Duration;

    fn current_track(&self) -> Option<&Track>;
}

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

    #[expect(dead_code)]
    stream: OutputStream,
    sink: Sink,
    mpris: Mpris,
    status: MusicPlayerStatus,

    playlist: Playlist,
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

            playlist: Playlist::new(Vec::new()),
            status: MusicPlayerStatus::Stopped,
        }
    }
}

impl GeneralMusicPlayer for MusicPlayer {
    fn play_track(&mut self, track: &Track) {
        self.sink.stop();

        if let Ok(file) = std::fs::File::open(track.path.as_path()) {
            self.mpris.set_metadata(MediaMetadata {
                album: track.album.as_deref(),
                title: track.title.as_deref(),
                artist: track.artist.as_deref(),
                duration: track.duration,
                cover_url: None,
            });
            self.sink
                .add(rodio::Decoder::try_from(file).expect("Audio samples."));
            self.sink.play();

            self.status = MusicPlayerStatus::Playing;

            self.player_tx.send(MusicPlayerEvent::PlaybackStarted).ok();
        }
    }

    #[inline]
    fn play_next(&mut self) {
        if let Some(track) = self.playlist.next_track().cloned() {
            self.play_track(&track);
        } else {
            self.stop();
        }
    }

    #[inline]
    fn play_previous(&mut self) {
        if self.position().as_millis() > 500 {
            self.seek(Duration::from_secs(0));
        } else if let Some(track) = self.playlist.previous_track().cloned() {
            self.play_track(&track);
        }
    }

    #[inline]
    fn playlist(&self) -> &Playlist {
        &self.playlist
    }

    #[inline]
    fn playlist_mut(&mut self) -> &mut Playlist {
        &mut self.playlist
    }

    #[inline]
    fn play(&mut self) {
        if self.sink.is_empty() {
            if let Some(track) = self.playlist.current_track().cloned() {
                self.play_track(&track);
            }
            return;
        }

        self.sink.play();
        self.status = MusicPlayerStatus::Playing;
    }

    #[inline]
    fn pause(&mut self) {
        if self.sink.is_empty() {
            return;
        }

        self.sink.pause();
        self.status = MusicPlayerStatus::Paused;
    }

    #[inline]
    fn stop(&mut self) {
        self.sink.stop();
        self.status = MusicPlayerStatus::Stopped;
    }

    #[inline]
    fn toggle(&mut self) {
        if self.is_paused() {
            self.play();
        } else {
            self.pause();
        }
    }

    #[inline]
    fn seek(&mut self, position: Duration) {
        self.sink.seek(position);
        self.mpris_update_progress();
    }

    #[inline]
    fn is_paused(&self) -> bool {
        self.sink.is_paused()
    }

    #[inline]
    fn is_stopped(&self) -> bool {
        self.sink.is_empty()
    }

    #[inline]
    fn set_volume(&mut self, value: f32) {
        self.mpris.set_volume(value.clamp(0.0, 1.0) as f64);
        self.sink.set_volume(value.clamp(0.0, 1.2));
    }

    #[inline]
    fn volume(&self) -> f32 {
        self.sink.volume()
    }

    #[inline]
    fn position(&self) -> Duration {
        self.sink.position()
    }

    #[inline]
    fn current_track(&self) -> Option<&Track> {
        self.playlist.current_track()
    }
}
