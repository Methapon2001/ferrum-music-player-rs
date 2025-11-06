use eframe::egui;
use eframe::egui::include_image;

use crate::{player::MusicPlayer, track::Track};

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
    player: &'a mut MusicPlayer,
}

impl<'a> TrackList<'a> {
    pub fn new(player: &'a mut MusicPlayer) -> Self {
        Self { player }
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
                .column(Column::initial(width * 0.1).at_least(48.0).clip(true))
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
                        .player
                        .playlist()
                        .tracks()
                        .iter()
                        .enumerate()
                        .filter(|(_index, item)| {
                            if state.search.is_empty() {
                                return true;
                            }
                            format!(
                                "{} {} {}",
                                item.album.as_deref().unwrap_or(""),
                                item.title.as_deref().unwrap_or(""),
                                item.artist.as_deref().unwrap_or(""),
                            )
                            .to_ascii_lowercase()
                            .trim()
                            .contains(&state.search.to_ascii_lowercase())
                        })
                        .collect::<Vec<(usize, &Track)>>();

                    // NOTE: To avoid tracks clone, store double clicked index and handle later.
                    let mut double_clicked_index: Option<usize> = None;

                    body.rows(24.0, tracks.len(), |mut row| {
                        let (index, item) = tracks[row.index()];

                        row.col(|ui| {
                            ui.centered_and_justified(|ui| {
                                if !self.player.is_stopped()
                                    && self
                                        .player
                                        .current_track()
                                        .is_some_and(|track| track.eq(item))
                                {
                                    ui.add(
                                        egui::Image::new(if self.player.is_paused() {
                                            include_image!("../../assets/icons/pause.svg")
                                        } else {
                                            include_image!("../../assets/icons/play.svg")
                                        })
                                        .max_size((16.0, 16.0).into()),
                                    );
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
                            double_clicked_index = Some(index);
                        }
                    });

                    if let Some(index) = double_clicked_index {
                        let track = self.player.playlist().tracks()[index].to_owned();

                        self.player.playlist_mut().select_track(index.to_owned());
                        self.player.play_track(track);
                    }
                });

            state.store(&ctx, id);
        })
        .response
    }
}
