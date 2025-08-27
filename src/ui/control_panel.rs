use std::time::Duration;

use eframe::egui::{self, Color32, Stroke, include_image};

use crate::player::MediaPlayer;

#[derive(Clone)]
struct State {
    volume: f32,
    duration: f32,
}

impl Default for State {
    fn default() -> Self {
        Self {
            volume: 1.0,
            duration: 0.0,
        }
    }
}

impl State {
    pub fn load(ctx: &egui::Context, id: egui::Id) -> Option<Self> {
        ctx.data_mut(|d| d.get_persisted(id))
    }

    pub fn store(self, ctx: &egui::Context, id: egui::Id) {
        ctx.data_mut(|d| d.insert_persisted(id, self));
    }
}

pub struct ControlPanel<'a> {
    player: &'a mut MediaPlayer,
}

impl<'a> ControlPanel<'a> {
    pub fn new(player: &'a mut MediaPlayer) -> Self {
        Self { player }
    }
}

impl egui::Widget for ControlPanel<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let id = ui.next_auto_id();
        let mut state = State::load(ui.ctx(), id).unwrap_or_default();

        ui.horizontal(|ui| {
            let slider_handle = egui::style::HandleShape::Rect { aspect_ratio: 0.5 };

            let toggle_button = ui.add_enabled(
                !self.player.is_empty(),
                egui::Button::new(if self.player.is_paused() || self.player.is_empty() {
                    (
                        egui::Image::new(include_image!("../../assets/icons/play.svg")),
                        "Play",
                    )
                } else {
                    (
                        egui::Image::new(include_image!("../../assets/icons/pause.svg")),
                        "Pause",
                    )
                })
                .fill(Color32::TRANSPARENT)
                .stroke(Stroke::NONE),
            );
            let stop_button = ui.add_enabled(
                !self.player.is_empty(),
                egui::Button::new((
                    egui::Image::new(include_image!("../../assets/icons/stop.svg")),
                    "Stop",
                ))
                .fill(Color32::TRANSPARENT)
                .stroke(Stroke::NONE),
            );

            match (toggle_button.clicked(), self.player.is_paused()) {
                (true, true) => {
                    self.player.play();
                }
                (true, false) => {
                    self.player.pause();
                }
                _ => {}
            }

            if stop_button.clicked() {
                self.player.stop();
            }

            ui.separator();

            ui.scope(|ui| {
                ui.spacing_mut().slider_width = 75.0;
                // TODO: Custom?
                let volume_slider = ui.add(
                    egui::Slider::new(&mut state.volume, 0.0..=1.0)
                        .handle_shape(slider_handle)
                        .show_value(false)
                        .step_by(0.02),
                );
                if volume_slider.dragged() {
                    self.player.set_volume(state.volume);
                }
            });

            ui.separator();

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // NOTE: Default to 1.0 so slider handle will be at the start.
                let total_duration = if let Some(track) = &self.player.get_track() {
                    track.duration.map(|t| t.as_secs_f32()).unwrap_or(1.0)
                } else {
                    1.0
                };

                state.duration = self.player.get_position().as_secs_f32();

                // TODO: Handle unknown total duration.
                if !self.player.is_empty() {
                    ui.ctx().request_repaint_after(Duration::from_millis(500));
                    ui.label(format!(
                        "{:02}:{:02} / {:02}:{:02}",
                        state.duration.trunc() as u64 / 60,
                        state.duration.trunc() as u64 % 60,
                        total_duration.trunc() as u64 / 60,
                        total_duration.trunc() as u64 % 60
                    ));
                } else {
                    ui.label("--:-- / --:--");
                }

                ui.scope(|ui| {
                    ui.spacing_mut().slider_width = ui.available_width();
                    let duration_slider = ui.add_enabled(
                        !self.player.is_empty(),
                        egui::Slider::new(&mut state.duration, 0.0..=total_duration)
                            .handle_shape(slider_handle)
                            .show_value(false)
                            .step_by(0.1),
                    );
                    if duration_slider.dragged() {
                        self.player.pause();
                        self.player.seek(Duration::from_secs_f32(state.duration))
                    }
                    if duration_slider.drag_stopped() {
                        self.player.play();
                    }
                });
            });

            state.store(ui.ctx(), id);
        })
        .response
    }
}
