use eframe::egui;
use eframe::egui::include_image;
use egui_extras::{Column, TableBuilder};

use crate::{player::MusicPlayer, track::Track};

#[derive(Default, Clone)]
struct State {
    search: String,
    selected_index: Option<usize>,
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
        let id = ui.next_auto_id();
        let mut state = State::load(ui.ctx(), id).unwrap_or_default();

        ui.vertical(|ui| {
            let mut widget_focused = false;
            ui.memory(|memory| {
                if memory.focused().is_some() {
                    widget_focused = true;
                }
            });

            let mut search_request = false;
            let mut select_changed = false;
            ui.input_mut(|input_state| {
                if !widget_focused {
                    if input_state.consume_key(egui::Modifiers::CTRL, egui::Key::F) {
                        search_request = true;
                    }
                    if input_state.consume_key(egui::Modifiers::NONE, egui::Key::Escape) {
                        state.selected_index = None;
                    }
                }
                if input_state.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp) {
                    if let Some(selected) = state.selected_index.as_mut() {
                        *selected = selected.saturating_sub(1);
                    } else {
                        state.selected_index = Some(usize::MAX);
                    }
                    select_changed = true;
                }
                if input_state.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown) {
                    if let Some(selected) = state.selected_index.as_mut() {
                        *selected = selected.saturating_add(1);
                    } else {
                        state.selected_index = Some(usize::MIN);
                    }
                    select_changed = true;
                }
            });

            let search_input = ui.add_sized(
                [ui.available_width(), 30.0],
                egui::TextEdit::singleline(&mut state.search)
                    .vertical_align(egui::Align::Center)
                    .hint_text("Search"),
            );
            if search_input.changed() {
                state.selected_index = None;
            }
            if search_request {
                search_input.request_focus();
            }

            let mut enter_pressed = false;
            ui.input_mut(|input_state| {
                if input_state.consume_key(egui::Modifiers::NONE, egui::Key::Enter) {
                    enter_pressed = true;
                }
            });

            let tracks = self
                .player
                .playlist()
                .tracks()
                .iter()
                .enumerate()
                .filter(|item| {
                    if state.search.is_empty() {
                        return true;
                    }
                    format!(
                        "{} {} {}",
                        item.1.album.as_deref().unwrap_or(""),
                        item.1.title.as_deref().unwrap_or(""),
                        item.1.artist.as_deref().unwrap_or(""),
                    )
                    .to_ascii_lowercase()
                    .trim()
                    .contains(&state.search.to_ascii_lowercase())
                })
                .collect::<Vec<(usize, &Track)>>();

            // NOTE: To avoid track clone, store to be act index and handle later.
            let mut action_index: Option<usize> = None;

            let width = ui.available_width();
            let mut table = TableBuilder::new(ui)
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
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center));

            let total = tracks.len();

            if !state.search.is_empty() && total == 1 {
                state.selected_index = Some(0);
            }
            if let Some(index) = state.selected_index.as_mut() {
                *index = index.to_owned().clamp(0, total.saturating_sub(1));

                if enter_pressed {
                    action_index = tracks.get(*index).map(|item| item.0);
                }
                if select_changed {
                    table = table.scroll_to_row(*index, None);
                }
            }

            table
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

                    body.rows(24.0, total, |mut row| {
                        let row_index = row.index();
                        let (playlist_index, playlist_track) = tracks[row_index];

                        if state.selected_index.is_some_and(|index| index == row_index) {
                            row.set_selected(true);
                        }

                        row.col(|ui| {
                            ui.centered_and_justified(|ui| {
                                if !self.player.is_stopped()
                                    && self
                                        .player
                                        .current_track()
                                        .is_some_and(|track| track.eq(playlist_track))
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
                            ui.label(playlist_track.album.as_deref().unwrap_or("-"));
                        });
                        row.col(|ui| {
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    let disc = playlist_track.disc.as_deref().unwrap_or_default();
                                    let track = playlist_track.track.as_deref().unwrap_or_default();

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
                            ui.label(playlist_track.title.as_deref().unwrap_or("-"));
                        });
                        row.col(|ui| {
                            ui.label(playlist_track.artist.as_deref().unwrap_or("-"));
                        });

                        if row.response().clicked() {
                            state.selected_index = Some(row_index);
                        }
                        if row.response().double_clicked() {
                            action_index = Some(playlist_index);
                        }
                    });
                });

            if let Some(index) = action_index {
                self.player.playlist_mut().select_track(index);
                if let Some(track) = self.player.playlist().current_track() {
                    self.player.play_track(track.to_owned());
                }
            }

            state.store(ui.ctx(), id);
        })
        .response
    }
}
