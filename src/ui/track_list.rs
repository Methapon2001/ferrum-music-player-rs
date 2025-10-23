use eframe::egui::{self, include_image};

use crate::{config::COVER_IMAGE_URI, player::MediaPlayer, track::Track};

#[derive(Default, Clone)]
struct State {
    search: String,
}

impl State {
    pub fn load(ctx: &egui::Context, id: egui::Id) -> Option<Self> {
        ctx.data_mut(|d| d.get_persisted(id))
    }

    pub fn store(self, ctx: &egui::Context, id: egui::Id) {
        ctx.data_mut(|d| d.insert_persisted(id, self));
    }
}

pub struct TrackList<'a> {
    player: &'a mut MediaPlayer,
    tracks: &'a [Track],
}

impl<'a> TrackList<'a> {
    pub fn new(player: &'a mut MediaPlayer, tracks: &'a [Track]) -> Self {
        Self { player, tracks }
    }
}

impl egui::Widget for TrackList<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        use egui_extras::{Column, TableBuilder};

        let ctx = ui.ctx().clone();

        let id = ui.next_auto_id();
        let mut state = State::load(&ctx, id).unwrap_or_default();

        ui.vertical(|ui| {
            let search_box_response = ui.add_sized(
                [ui.available_width(), 30.0],
                egui::TextEdit::singleline(&mut state.search)
                    .vertical_align(egui::Align::Center)
                    .hint_text("Search"),
            );

            let mut widget_focused = false;

            ui.ctx().memory(|memory| {
                if memory.focused().is_some() {
                    widget_focused = true;
                }
            });

            if !widget_focused {
                let mut search_box_request_focus = false;

                ui.ctx().input_mut(|input_state| {
                    if input_state.consume_key(egui::Modifiers::CTRL, egui::Key::F) {
                        search_box_request_focus = true;
                    }
                });

                if search_box_request_focus {
                    search_box_response.request_focus();
                }
            }

            let width = ui.available_width();

            TableBuilder::new(ui)
                .sense(egui::Sense::click())
                .striped(true)
                .resizable(true)
                .auto_shrink(false)
                .column(Column::initial(width * 0.1).at_least(30.0).clip(true))
                .column(
                    Column::initial(width * 0.3)
                        .at_least(width * 0.2)
                        .clip(true),
                )
                .column(Column::initial(width * 0.15).at_least(50.0).clip(true))
                .column(
                    Column::initial(width * 0.3)
                        .at_least(width * 0.2)
                        .clip(true),
                )
                .column(Column::remainder().clip(true))
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .header(32.0, |mut header| {
                    header.col(|ui| {
                        ui.centered_and_justified(|ui| {
                            ui.strong("Playing");
                        });
                    });
                    header.col(|ui| {
                        ui.strong("Album");
                    });
                    header.col(|ui| {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.strong("Track No.");
                        });
                    });
                    header.col(|ui| {
                        ui.strong("Title");
                    });
                    header.col(|ui| {
                        ui.strong("Artist");
                    });
                })
                .body(|mut body| {
                    body.ui_mut().style_mut().interaction.selectable_labels = false;

                    let tracks = self
                        .tracks
                        .iter()
                        .filter(|item| {
                            if state.search.is_empty() {
                                return true;
                            }

                            format!(
                                "{} {} {}",
                                item.album.as_deref().unwrap_or(""),
                                item.title.as_deref().unwrap_or(""),
                                item.artist.as_deref().unwrap_or(""),
                            )
                            .to_lowercase()
                            .trim()
                            .contains(&state.search.to_lowercase())
                        })
                        .collect::<Vec<&Track>>();

                    body.rows(24.0, tracks.len(), |mut row| {
                        let item = tracks[row.index()];

                        row.col(|ui| {
                            ui.centered_and_justified(|ui| {
                                if !self.player.is_empty() && self.player.is_playing_track(item) {
                                    ui.image(if self.player.is_paused() {
                                        include_image!("../../assets/icons/pause.svg")
                                    } else {
                                        include_image!("../../assets/icons/play.svg")
                                    });
                                }
                            });
                        });
                        row.col(|ui| {
                            ui.label(item.album.as_deref().unwrap_or("-"));
                        });
                        row.col(|ui| {
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    let disc = item.disc.as_deref().unwrap_or_default();
                                    let track = item.track.as_deref().unwrap_or_default();

                                    match (disc.is_empty(), track.is_empty()) {
                                        (false, false) => {
                                            ui.label(format!("{}.{:0>2}", disc, track));
                                        }
                                        (true, false) => {
                                            ui.label(format!("{:0>2}", track));
                                        }
                                        _ => {}
                                    }
                                },
                            );
                        });
                        row.col(|ui| {
                            ui.label(item.title.as_deref().unwrap_or("-"));
                        });
                        row.col(|ui| {
                            ui.label(item.artist.as_deref().unwrap_or("-"));
                        });

                        if row.response().double_clicked() {
                            let mut track = item.to_owned();

                            if let Ok(front_cover) = track.read_front_cover() {
                                track.cover = front_cover;
                            }

                            if let Some(current_track) = self.player.get_track()
                                && current_track.cover != track.cover
                            {
                                ctx.forget_image(COVER_IMAGE_URI);
                            }

                            self.player.add(track);
                            self.player.play();
                        }
                    });
                });

            state.store(&ctx, id);
        })
        .response
    }
}
