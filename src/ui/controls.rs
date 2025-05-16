use std::time::Duration;

use eframe::egui;

use crate::common::TrackInfo;

#[derive(Clone)]
pub struct ControllerState {
    volume: f32,
    duration: f32,
}

impl Default for ControllerState {
    fn default() -> Self {
        Self {
            volume: 1.0,
            duration: 0.0,
        }
    }
}

impl ControllerState {
    pub fn load(ctx: &egui::Context, id: egui::Id) -> Option<Self> {
        ctx.data_mut(|d| d.get_persisted(id))
    }

    pub fn store(self, ctx: &egui::Context, id: egui::Id) {
        ctx.data_mut(|d| d.insert_persisted(id, self));
    }
}

/// A user interface controller for music playback.
///
/// This struct provides the necessary components to control audio playback
/// and display relevant track information.
///
/// It holds references to:
/// - `audio_sink`: A [`rodio::Sink`](https://docs.rs/rodio/0.20.1/rodio/struct.Sink.html) for managing audio playback controls (play, pause, volume, etc.).
/// - `track_info`: An optional [`TrackInfo`](#struct.TrackInfo) struct containing metadata for the currently playing track.
#[derive(Clone)]
pub struct Controller<'a> {
    audio_sink: &'a rodio::Sink,
    track_info: &'a Option<TrackInfo>,
}

impl<'a> Controller<'a> {
    pub fn new(audio_sink: &'a rodio::Sink, track_info: &'a Option<TrackInfo>) -> Self {
        Self {
            track_info,
            audio_sink,
        }
    }
}

impl egui::Widget for Controller<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let id = ui.next_auto_id();
        let mut state = ControllerState::load(ui.ctx(), id).unwrap_or_default();

        ui.horizontal(|ui| {
            let slider_handle = egui::style::HandleShape::Rect { aspect_ratio: 0.5 };

            // TODO: New layout and use icon.
            let play_button = ui.add_enabled(
                self.audio_sink.is_paused() && !self.audio_sink.empty(),
                egui::Button::new("Play"),
            );
            let pause_button = ui.add_enabled(
                !self.audio_sink.is_paused() && !self.audio_sink.empty(),
                egui::Button::new("Pause"),
            );
            let stop_button = ui.add_enabled(!self.audio_sink.empty(), egui::Button::new("Stop"));

            if play_button.clicked() {
                self.audio_sink.play();
            }
            if pause_button.clicked() {
                self.audio_sink.pause();
            }
            if stop_button.clicked() {
                self.audio_sink.clear();
            }

            ui.separator();

            {
                ui.spacing_mut().slider_width = 75.0;
                // TODO: Custom?
                let volume_slider = ui.add(
                    egui::Slider::new(&mut state.volume, 0.0..=1.0)
                        .handle_shape(slider_handle)
                        .show_value(false)
                        .step_by(0.1),
                );
                if volume_slider.dragged() {
                    self.audio_sink.set_volume(state.volume);
                }
            }

            ui.separator();

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // NOTE: Default to 1.0 so slider handle will be at the start.
                let total_duration = if let Some(track) = &self.track_info {
                    track.total_duration.map(|t| t.as_secs_f32()).unwrap_or(1.0)
                } else {
                    1.0
                };

                state.duration = self.audio_sink.get_pos().as_secs_f32();

                // TODO: Handle unknown total duration.
                if !self.audio_sink.empty() {
                    ui.ctx().request_repaint_after(Duration::from_millis(100));
                    ui.label(format!(
                        "{:02}:{:02} / {:02}:{:02}",
                        state.duration.trunc() as u64 / 60,
                        state.volume.trunc() as u64 % 60,
                        total_duration.trunc() as u64 / 60,
                        total_duration.trunc() as u64 % 60
                    ));
                } else {
                    ui.label("--:-- / --:--");
                }

                {
                    ui.spacing_mut().slider_width = ui.available_width();
                    let duration_slider = ui.add_enabled(
                        !self.audio_sink.empty(),
                        egui::Slider::new(&mut state.duration, 0.0..=total_duration)
                            .handle_shape(slider_handle)
                            .show_value(false),
                    );
                    if duration_slider.dragged() {
                        self.audio_sink.pause();
                        self.audio_sink
                            .try_seek(Duration::from_secs_f32(state.duration))
                            .unwrap();
                    }
                    if duration_slider.drag_stopped() {
                        self.audio_sink.play();
                    }
                }
            });

            state.store(ui.ctx(), id);
        })
        .response
    }
}
