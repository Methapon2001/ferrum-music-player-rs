use eframe::egui;
use eframe::egui::{Id, include_image};
use egui_extras::{Column, TableBuilder};

use crate::track::Track;

pub type TrackIndex = usize;

#[derive(Debug, Clone)]
pub enum TrackListAction {
    Play(TrackIndex),
    Select(TrackIndex),

    SendToCurrentPlaylist(Vec<TrackIndex>),
}

#[derive(Debug, Clone, Copy)]
pub enum TrackListIndicator {
    Playing(TrackIndex),
    Paused(TrackIndex),
}

#[derive(Debug, Clone)]
pub enum TrackListContextMenu {
    SendToCurrentPlaylist,
}

#[derive(Default, Clone)]
struct State {
    scroll_position: f32,
    search_input: String,
    selected_index: Option<TrackIndex>,
}

impl State {
    pub fn load(ctx: &egui::Context, id: Id) -> Option<Self> {
        ctx.data_mut(|d| d.get_persisted(id))
    }

    pub fn store(self, ctx: &egui::Context, id: Id) {
        ctx.data_mut(|d| d.insert_persisted(id, self));
    }
}

pub struct TrackList<'a> {
    id: Id,

    action: &'a mut Option<TrackListAction>,
    tracks: &'a [Track],
    indicator: Option<TrackListIndicator>,

    context_menu: Vec<TrackListContextMenu>,
}

impl<'a> TrackList<'a> {
    pub fn new(
        action: &'a mut Option<TrackListAction>,
        tracks: &'a [Track],
        indicator: Option<TrackListIndicator>,
        id: impl Into<Id>,
    ) -> Self {
        Self {
            id: id.into(),

            action,
            tracks,
            indicator,

            context_menu: Vec::new(),
        }
    }

    pub fn context_menu(mut self, menus: Vec<TrackListContextMenu>) -> Self {
        self.context_menu = menus;
        self
    }
}

impl egui::Widget for TrackList<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let mut state = State::load(ui.ctx(), self.id).unwrap_or_default();

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
                        state.selected_index = Some(TrackIndex::MAX);
                    }
                    select_changed = true;
                }
                if input_state.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown) {
                    if let Some(selected) = state.selected_index.as_mut() {
                        *selected = selected.saturating_add(1);
                    } else {
                        state.selected_index = Some(TrackIndex::MIN);
                    }
                    select_changed = true;
                }
            });

            let search_input = ui.add_sized(
                [ui.available_width(), 30.0],
                egui::TextEdit::singleline(&mut state.search_input)
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

            ui.separator();

            let tracks = self
                .tracks
                .iter()
                .enumerate()
                .filter(|item| {
                    if state.search_input.is_empty() {
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
                    .contains(&state.search_input.to_ascii_lowercase())
                })
                .collect::<Vec<(TrackIndex, &Track)>>();

            // NOTE: To avoid track clone, store to be act index and handle later.
            let mut action_index: Option<TrackIndex> = None;

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

            if !state.search_input.is_empty() && total == 1 {
                state.selected_index = Some(0);
            }
            if let Some(index) = state.selected_index.as_mut() {
                *index = index.to_owned().clamp(0, total.saturating_sub(1));

                if enter_pressed {
                    action_index = tracks.get(*index).map(|item| item.0);
                }
                if select_changed {
                    table = table.scroll_to_row(*index, None);
                } else {
                    table = table.vertical_scroll_offset(state.scroll_position);
                }
            }

            let scroll_output = table
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

                        let Some(item) = tracks.get(row_index).copied() else {
                            return;
                        };

                        let (item_index, item) = item;

                        if state.selected_index.is_some_and(|index| index == row_index) {
                            row.set_selected(true);
                        }

                        row.col(|ui| {
                            ui.centered_and_justified(|ui| {
                                if let Some(indicator) = self.indicator.as_ref() {
                                    let image_size = (16.0, 16.0);

                                    match indicator {
                                        TrackListIndicator::Playing(index) => {
                                            if item_index.eq(index) {
                                                ui.add(
                                                    egui::Image::new(include_image!(
                                                        "../../assets/icons/play.svg"
                                                    ))
                                                    .max_size(image_size.into()),
                                                );
                                            }
                                        }
                                        TrackListIndicator::Paused(index) => {
                                            if item_index.eq(index) {
                                                ui.add(
                                                    egui::Image::new(include_image!(
                                                        "../../assets/icons/pause.svg"
                                                    ))
                                                    .max_size(image_size.into()),
                                                );
                                            }
                                        }
                                    }
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
                                            ui.label(format!("{disc}.{track:0>2}"));
                                        }
                                        (true, false) => {
                                            ui.label(format!("{track:0>2}"));
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

                        if !self.context_menu.is_empty() {
                            row.response().context_menu(|ui| {
                                let mut send_to_queue = None;

                                for menu in &self.context_menu {
                                    match menu {
                                        TrackListContextMenu::SendToCurrentPlaylist => {
                                            send_to_queue =
                                                Some(egui::Button::new("Send to current playlist"));
                                        }
                                    }
                                }

                                if let Some(send_to_queue) = send_to_queue
                                    && ui.add(send_to_queue).clicked()
                                {
                                    *self.action =
                                        Some(TrackListAction::SendToCurrentPlaylist(vec![
                                            item_index,
                                        ]));
                                }
                            });
                        }

                        if row.response().clicked() || row.response().secondary_clicked() {
                            state.selected_index = Some(row_index);
                            select_changed = true;
                        }

                        if row.response().double_clicked() {
                            action_index = Some(item_index);
                        }
                    });
                });

            state.scroll_position = scroll_output.state.offset.y;

            if select_changed {
                *self.action = state.selected_index.map(TrackListAction::Select);
            }

            if action_index.is_some() {
                *self.action = action_index.map(TrackListAction::Play);
            }

            state.store(ui.ctx(), self.id);
        })
        .response
    }
}
