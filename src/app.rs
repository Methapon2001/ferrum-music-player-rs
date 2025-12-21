use std::io;
use std::sync::Arc;
use std::sync::mpsc;
use std::thread;

use eframe::egui;
use eframe::egui::TextureHandle;
use egui_extras::{Size, StripBuilder};
use log::debug;
use parking_lot::Mutex;

use crate::config::{COVER_IMAGE_SIZE, get_default_app_dir_config, get_font_definitions};
use crate::database::{Database, get_all_tracks};
use crate::player::{GeneralMusicPlayer as _, MusicPlayer, MusicPlayerEvent};
use crate::playlist::{Playlist, PlaylistId};
use crate::track::Track;
use crate::ui::control_panel::ControlPanel;
use crate::ui::track_list::TrackListItemMenu;
use crate::ui::track_list::{TrackList, TrackListAction, TrackListIndicator};

enum TrackListView {
    Library,
    Playlist(Option<PlaylistId>),
}

pub struct App {
    player: Arc<Mutex<MusicPlayer>>,
    library: Arc<Mutex<Vec<Track>>>,
    cover: Arc<Mutex<Option<TextureHandle>>>,

    current_track_list_view: TrackListView,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        egui_extras::install_image_loaders(&cc.egui_ctx);

        cc.egui_ctx.set_fonts(get_font_definitions());
        cc.egui_ctx.options_mut(|options| {
            options.input_options.line_scroll_speed = 100.0;
        });

        let (player_tx, player_rx) = mpsc::channel();
        let player = Arc::new(Mutex::new(MusicPlayer::new(player_tx)));
        let library = Arc::new(Mutex::new(Vec::new()));
        let cover = Arc::new(Mutex::new(None));

        {
            let player = player.clone();
            let library = library.clone();
            let cover = cover.clone();
            let ctx = cc.egui_ctx.clone();

            thread::spawn(move || -> ! {
                let database = Database::new().expect("Database connected.");

                database.refresh_library(false).ok();

                let tracks = get_all_tracks(&database.get_connection()).unwrap_or_default();

                *library.lock() = tracks;

                match Playlist::new_from_file(&get_default_app_dir_config().join("default.m3u")) {
                    Ok(playlist) => {
                        *player.lock().playlist_mut() = playlist;
                    }
                    Err(err) => {
                        if err.kind() == io::ErrorKind::NotFound {
                            debug!("Current playlist not found.");
                        } else {
                            debug!("{err:?}");
                        }
                    }
                }

                ctx.request_repaint();

                loop {
                    if let Ok(player_event) = player_rx.recv() {
                        match player_event {
                            MusicPlayerEvent::Tick => {
                                let mut player = player.lock();
                                if let Some(mpris_event) = player.mpris_event() {
                                    player.mpris_handle(&mpris_event);
                                }
                                ctx.request_repaint();
                            }
                            MusicPlayerEvent::PlaybackStarted => {
                                let track = player.lock().current_track().cloned();

                                let texture = track.and_then(|t| match t.read_front_cover() {
                                    Ok(front_cover) => {
                                        let buffer = front_cover.as_deref()?;

                                        image::load_from_memory(buffer)
                                            .map(|image| {
                                                let size =
                                                    [image.width() as _, image.height() as _];
                                                let image_buffer = image.to_rgba8();
                                                let pixels = image_buffer.as_flat_samples();

                                                ctx.load_texture(
                                                    "cover",
                                                    egui::ColorImage::from_rgba_unmultiplied(
                                                        size,
                                                        pixels.as_slice(),
                                                    ),
                                                    egui::TextureOptions::default(),
                                                )
                                            })
                                            .ok()
                                    }
                                    Err(_) => None,
                                });

                                *cover.lock() = texture;

                                ctx.request_repaint();
                            }
                            MusicPlayerEvent::PlaybackProgress => {
                                player.lock().mpris_update_progress();
                            }
                            MusicPlayerEvent::PlaybackEnded => {
                                player.lock().play_next();
                                // NOTE: Repaint is needed after doing something with playlist and
                                // player so that the UI state isn't stale.
                                ctx.request_repaint();
                            }
                            MusicPlayerEvent::PlaybackStopped => {}
                        }
                    }
                }
            });
        }

        Self {
            player,
            library,
            cover,

            current_track_list_view: TrackListView::Library,
        }
    }

    pub fn panel(&mut self, ui: &mut egui::Ui) {
        let mut player = self.player.lock();

        ui.add(ControlPanel::new(&mut *player));

        // TODO: Scan progress.
    }

    pub fn meta(&mut self, ui: &mut egui::Ui) {
        let player = self.player.lock();

        StripBuilder::new(ui)
            .size(Size::remainder())
            .size(Size::exact(1.0))
            .size(Size::remainder())
            .vertical(|mut strip| {
                strip.cell(|ui| {
                    let mut cursor = ui.cursor();

                    cursor.set_width(COVER_IMAGE_SIZE.0);
                    cursor.set_height(COVER_IMAGE_SIZE.1);

                    let style = ui.ctx().style();

                    ui.painter().rect_filled(
                        cursor,
                        style.noninteractive().corner_radius,
                        style.visuals.extreme_bg_color,
                    );

                    ui.add_sized(
                        COVER_IMAGE_SIZE,
                        if !player.is_stopped()
                            && let Some(cover) = self.cover.lock().as_ref()
                        {
                            egui::Image::from_texture(cover)
                        } else {
                            egui::Image::new(egui::include_image!(
                                "../assets/album-placeholder.png"
                            ))
                        }
                        .shrink_to_fit(),
                    )
                    .on_hover_text_at_pointer("Cover Image");

                    if let Some(current_track) = player.current_track()
                        && !player.is_stopped()
                    {
                        ui.horizontal(|ui| {
                            ui.vertical_centered(|ui| {
                                match (
                                    current_track.album.as_deref(),
                                    current_track.title.as_deref(),
                                ) {
                                    (Some(album), Some(title)) => {
                                        ui.heading(title);
                                        ui.label(album);
                                    }
                                    (Some(album), None) => {
                                        ui.label(album);
                                    }
                                    (None, Some(title)) => {
                                        ui.label(title);
                                    }
                                    (None, None) => {
                                        ui.label("Unknown");
                                    }
                                }
                            });
                        });
                    }
                });
                strip.cell(|ui| {
                    ui.separator();
                });
                strip.cell(|ui| {
                    ui.vertical_centered_justified(|ui| {
                        let library_button = ui.add(egui::Button::new("Library"));
                        let playlist_button = ui.add(egui::Button::new("Default Playlist"));

                        if library_button.clicked() {
                            self.current_track_list_view = TrackListView::Library;
                        }
                        if playlist_button.clicked() {
                            self.current_track_list_view = TrackListView::Playlist(None);
                        }
                    });
                });
            });
    }

    pub fn body(&mut self, ui: &mut egui::Ui) {
        let mut player = self.player.lock();

        let mut action = None;
        let mut indicator = None;

        if !player.is_stopped() {
            let playlist = player.playlist();
            let playing_playlist_id = playlist.get_id().map(|item| item.to_owned());
            let current_playing_index = playlist.current_track_index();

            match &self.current_track_list_view {
                TrackListView::Playlist(current_id) => {
                    if playing_playlist_id.eq(current_id) {
                        if player.is_paused() {
                            indicator = Some(TrackListIndicator::Paused(current_playing_index));
                        } else {
                            indicator = Some(TrackListIndicator::Playing(current_playing_index));
                        }
                    }
                }
                TrackListView::Library => {}
            }

            if let TrackListView::Playlist(current_id) = &self.current_track_list_view
                && playing_playlist_id.eq(current_id)
            {
                if player.is_paused() {
                    indicator = Some(TrackListIndicator::Paused(current_playing_index));
                } else {
                    indicator = Some(TrackListIndicator::Playing(current_playing_index));
                }
            }
        }

        match &self.current_track_list_view {
            TrackListView::Library => {
                let library = self.library.lock();

                ui.add(
                    TrackList::new(&mut action, library.as_slice(), indicator)
                        .item_menu(vec![TrackListItemMenu::SendToQueue]),
                );

                if let Some(action) = action {
                    match action {
                        TrackListAction::Play(index) => {
                            player.playlist_mut().clear();
                            player.playlist_mut().push(library[index].clone());

                            player.stop();
                            player.play();
                        }
                        TrackListAction::Select(_index) => {}
                        TrackListAction::Edit(_index) => {}

                        TrackListAction::SendToQueue(indexes) => {
                            for index in indexes {
                                player.playlist_mut().push(library[index].clone());
                            }
                        }
                    }
                }
            }
            TrackListView::Playlist(view_playlist_id) => {
                let tracks = player.playlist().tracks();

                ui.add(TrackList::new(&mut action, tracks, indicator));

                if let Some(action) = action {
                    match action {
                        TrackListAction::Play(index) => {
                            player.playlist_mut().select_track(index);

                            player.stop();
                            player.play();
                        }
                        TrackListAction::Select(_index) => {}
                        TrackListAction::Edit(_index) => {}

                        TrackListAction::SendToQueue(_indexes) => {}
                    }
                }
            }
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let frame = egui::frame::Frame::new()
            .fill(ctx.style().visuals.panel_fill)
            .inner_margin(12);

        egui::TopBottomPanel::bottom("controls")
            .frame(frame)
            .show(ctx, |ui| self.panel(ui));
        egui::SidePanel::left("meta")
            .frame(frame)
            .show(ctx, |ui| self.meta(ui));
        egui::CentralPanel::default()
            .frame(frame)
            .show(ctx, |ui| self.body(ui));
    }
}
