use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::time::Duration;

use log::warn;
use parking_lot::Mutex;
use rodio::Source;
use rodio::mixer::Mixer;
use rodio::queue;

use super::MusicPlayerEvent;
use super::source::DoneCallback;

struct Controls {
    pause: AtomicBool,
    stopped: AtomicBool,
    volume: Mutex<f32>,
    position: Mutex<Duration>,
    seek: Mutex<Option<Duration>>,
}

/// Handle to a device that outputs sounds.
///
/// Dropping the `Sink` stops all sounds.
pub(super) struct Sink {
    player_tx: Sender<MusicPlayerEvent>,

    queue: Arc<queue::SourcesQueueInput>,
    controls: Arc<Controls>,

    sleep_until_end: Mutex<Option<Receiver<()>>>,
}

impl Sink {
    pub fn new(mixer: &Mixer, player_tx: Sender<MusicPlayerEvent>) -> Self {
        // TODO: Create custom queue to support source modification (e.g., crossfade)
        let (queue, source) = queue::queue(true);

        mixer.add(source);

        Self {
            player_tx,

            controls: Arc::new(Controls {
                pause: AtomicBool::new(false),
                stopped: AtomicBool::new(true),

                seek: Mutex::new(None),
                volume: Mutex::new(1.0),
                position: Mutex::new(Duration::ZERO),
            }),
            queue,

            sleep_until_end: Mutex::new(None),
        }
    }

    /// Add sound to sink and play if stopped or else add to queue.
    pub fn add<S>(&self, source: S)
    where
        S: Source + Send + 'static,
    {
        // NOTE: Wait for the queue to flush then resume stopped playback
        if self.controls.stopped.load(Ordering::SeqCst) {
            self.sleep_until_end();
            self.controls.stopped.store(false, Ordering::SeqCst);
        }

        let player_tx = self.player_tx.clone();
        let controls = self.controls.clone();
        let source = source
            .track_position()
            .pausable(false)
            .amplify(1.0)
            .skippable()
            .periodic_access(Duration::from_millis(5), move |s| {
                if controls.stopped.load(Ordering::SeqCst) {
                    s.skip();
                    *controls.position.lock() = Duration::ZERO;
                }

                let amplify = s.inner_mut();
                amplify.set_factor(*controls.volume.lock());

                let pausable = amplify.inner_mut();
                pausable.set_paused(controls.pause.load(Ordering::SeqCst));

                let track_position = pausable.inner_mut();
                *controls.position.lock() = track_position.get_pos();

                if let Some(err) = controls
                    .seek
                    .lock()
                    .take()
                    .and_then(|seek| s.try_seek(seek).err())
                {
                    warn!("Seek error: {err:?}");
                }
            })
            .periodic_access(Duration::from_millis(500), move |_| {
                player_tx.send(MusicPlayerEvent::PlaybackProgress).ok();
            });

        let controls = self.controls.clone();
        let player_tx = self.player_tx.clone();
        let source = DoneCallback::new(source, move || {
            if controls.stopped.load(Ordering::SeqCst) {
                player_tx.send(MusicPlayerEvent::PlaybackStopped).ok();
            } else {
                player_tx.send(MusicPlayerEvent::PlaybackEnded).ok();
            }

            controls.stopped.store(true, Ordering::SeqCst);
        });

        *self.sleep_until_end.lock() = Some(self.queue.append_with_signal(source));
    }

    #[inline]
    pub fn stop(&self) {
        self.controls.stopped.store(true, Ordering::SeqCst);
    }

    #[inline]
    pub fn seek(&self, position: Duration) {
        *self.controls.seek.lock() = Some(position);
    }

    #[inline]
    pub fn play(&self) {
        self.controls.pause.store(false, Ordering::SeqCst);
    }

    #[inline]
    pub fn pause(&self) {
        self.controls.pause.store(true, Ordering::SeqCst);
    }

    #[inline]
    pub fn is_paused(&self) -> bool {
        self.controls.pause.load(Ordering::SeqCst)
    }

    #[inline]
    pub fn volume(&self) -> f32 {
        *self.controls.volume.lock()
    }

    #[inline]
    pub fn set_volume(&self, value: f32) {
        *self.controls.volume.lock() = value;
    }

    #[inline]
    pub fn position(&self) -> Duration {
        *self.controls.position.lock()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.controls.stopped.load(Ordering::SeqCst)
    }

    #[inline]
    pub fn sleep_until_end(&self) {
        if let Some(sleep_until_end) = self.sleep_until_end.lock().take() {
            sleep_until_end.recv().ok();
        }
    }
}

impl Drop for Sink {
    #[inline]
    fn drop(&mut self) {
        self.queue.set_keep_alive_if_empty(false);
        self.controls.stopped.store(true, Ordering::Relaxed);
    }
}
