use std::time::Duration;

use eframe::egui::{self, Color32, Stroke, include_image};

use crate::{player::GeneralMusicPlayer, playlist::PlaylistMode};

#[derive(Clone)]
struct State {
    volume: f32,
    duration: f32,
    seek: bool,
    seek_while_playing: bool,
}

impl Default for State {
    fn default() -> Self {
        Self {
            volume: 1.0,
            duration: 0.0,
            seek: false,
            seek_while_playing: false,
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

pub struct ControlPanel<'a, T: GeneralMusicPlayer> {
    player: &'a mut T,
}

impl<'a, T: GeneralMusicPlayer> ControlPanel<'a, T> {
    pub fn new(player: &'a mut T) -> Self {
        Self { player }
    }
}

impl<T: GeneralMusicPlayer> egui::Widget for ControlPanel<'_, T> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let id = ui.next_auto_id();
        let mut state = State::load(ui.ctx(), id).unwrap_or_default();

        if !self.player.is_stopped() {
            let mut widget_focused = false;

            ui.memory(|memory| {
                if memory.focused().is_some() {
                    widget_focused = true;
                }
            });

            if !widget_focused {
                ui.input_mut(|input_state| {
                    if input_state.consume_key(egui::Modifiers::NONE, egui::Key::Space) {
                        if self.player.is_stopped() {
                            return;
                        }

                        self.player.toggle();
                    }
                    if input_state.consume_key(egui::Modifiers::NONE, egui::Key::ArrowLeft) {
                        self.player.seek(Duration::from_secs_f32(
                            (state.duration - 5.0).clamp(0.0, state.duration),
                        ));
                    }
                    if input_state.consume_key(egui::Modifiers::NONE, egui::Key::ArrowRight) {
                        self.player
                            .seek(Duration::from_secs_f32(state.duration + 5.0));
                    }
                });
            }
            state.duration = self.player.position().as_secs_f32();
        } else {
            state.duration = 0.0;
        }

        ui.horizontal(|ui| {
            let slider_handle = egui::style::HandleShape::Rect { aspect_ratio: 0.5 };

            ui.scope(|ui| {
                ui.style_mut().visuals.widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
                ui.style_mut().spacing.item_spacing = egui::Vec2::new(4., 4.);

                let stop_button = ui.add_enabled(
                    !self.player.is_stopped(),
                    egui::Button::new(egui::Image::new(include_image!(
                        "../../assets/icons/stop.svg"
                    )))
                    .stroke(Stroke::NONE),
                );
                let skip_backward_button = ui.add_enabled(
                    !self.player.is_stopped(),
                    egui::Button::new(egui::Image::new(include_image!(
                        "../../assets/icons/skip-backward.svg"
                    )))
                    .stroke(Stroke::NONE),
                );
                let toggle_button = ui.add_enabled(
                    !self.player.is_stopped(),
                    egui::Button::new(
                        if (self.player.is_paused() || self.player.is_stopped())
                            && !(state.seek && state.seek_while_playing)
                        {
                            egui::Image::new(include_image!("../../assets/icons/play.svg"))
                        } else {
                            egui::Image::new(include_image!("../../assets/icons/pause.svg"))
                        },
                    )
                    .stroke(Stroke::NONE),
                );
                let skip_forward_button = ui.add_enabled(
                    !self.player.is_stopped(),
                    egui::Button::new(egui::Image::new(include_image!(
                        "../../assets/icons/skip-forward.svg"
                    )))
                    .stroke(Stroke::NONE),
                );
                let mode_button = ui.add(
                    egui::Button::new(match self.player.playlist().mode() {
                        PlaylistMode::NoRepeat => {
                            egui::Image::new(include_image!("../../assets/icons/no-repeat.svg"))
                        }
                        PlaylistMode::Repeat => {
                            egui::Image::new(include_image!("../../assets/icons/repeat.svg"))
                        }
                        PlaylistMode::RepeatSingle => {
                            egui::Image::new(include_image!("../../assets/icons/repeat-one.svg"))
                        }
                        PlaylistMode::Random => {
                            egui::Image::new(include_image!("../../assets/icons/shuffle.svg"))
                        }
                    })
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
                if skip_backward_button.clicked() {
                    self.player.play_previous();
                }
                if skip_forward_button.clicked() {
                    self.player.play_next();
                }
                if mode_button.clicked() {
                    let playlist = self.player.playlist_mut();

                    match playlist.mode() {
                        PlaylistMode::NoRepeat => playlist.set_mode(PlaylistMode::Repeat),
                        PlaylistMode::Repeat => playlist.set_mode(PlaylistMode::RepeatSingle),
                        PlaylistMode::RepeatSingle => playlist.set_mode(PlaylistMode::Random),
                        PlaylistMode::Random => playlist.set_mode(PlaylistMode::NoRepeat),
                    }
                }
            });

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
                } else {
                    state.volume = self.player.volume();
                }
            });

            ui.separator();

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // NOTE: Default to 1.0 so slider handle will be at the start.
                let total_duration = if let Some(track) = &self.player.current_track() {
                    track.duration.map(|t| t.as_secs_f32()).unwrap_or(1.0)
                } else {
                    1.0
                };

                // TODO: Handle unknown total duration.
                if !self.player.is_stopped() {
                    ui.ctx().request_repaint_after(Duration::from_millis(150));
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
                        !self.player.is_stopped(),
                        egui::Slider::new(&mut state.duration, 0.0..=total_duration)
                            .handle_shape(slider_handle)
                            .show_value(false)
                            .step_by(0.1),
                    );
                    if duration_slider.drag_started() {
                        state.seek = true;
                        state.seek_while_playing = !self.player.is_paused();
                    }
                    if duration_slider.dragged() {
                        self.player.pause();
                        self.player.seek(Duration::from_secs_f32(state.duration));
                    }
                    if duration_slider.drag_stopped() {
                        state.seek = false;

                        if state.seek_while_playing {
                            self.player.play();
                        }
                    }
                });
            });

            state.store(ui.ctx(), id);
        })
        .response
    }
}
