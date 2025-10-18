use std::{
    sync::{Arc, mpsc},
    thread,
};

use eframe::egui::{
    self, CornerRadius, Visuals,
    style::{WidgetVisuals, Widgets},
};
use parking_lot::Mutex;

use crate::{
    config::{COVER_IMAGE_SIZE, COVER_IMAGE_URI, get_font_definitions},
    database::{Database, get_all_tracks},
    player::{MusicPlayer, MusicPlayerEvent},
    playlist::Playlist,
    ui::{control_panel::ControlPanel, track_list::TrackList},
};

pub struct App {
    player: Arc<Mutex<MusicPlayer>>,
    playlist: Arc<Mutex<Playlist>>,
    cover: Arc<Mutex<Option<Vec<u8>>>>,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> App {
        egui_extras::install_image_loaders(&cc.egui_ctx);

        cc.egui_ctx.set_fonts(get_font_definitions());
        cc.egui_ctx.options_mut(|options| {
            options.input_options.line_scroll_speed = 100.0;
        });

        let (player_tx, player_rx) = mpsc::channel();
        let player = Arc::new(Mutex::new(MusicPlayer::new(player_tx)));
        let playlist = Arc::new(Mutex::new(Playlist::new(Vec::new())));
        let cover = Arc::new(Mutex::new(None));

        {
            let player = player.clone();
            let playlist = playlist.clone();
            let cover = cover.clone();
            let ctx = cc.egui_ctx.clone();

            thread::spawn(move || {
                let database = Database::new().expect("Database connected.");

                database.refresh_library(false).ok();

                // NOTE: Default playlist is the library.
                // TODO: Separate library and playlist.
                *playlist.lock() =
                    Playlist::new(get_all_tracks(&database.get_connection()).unwrap_or_default());

                ctx.request_repaint();

                // TODO: Load playlist(s).

                loop {
                    if let Ok(player_event) = player_rx.recv() {
                        let mut player = player.lock();

                        match player_event {
                            MusicPlayerEvent::Tick => {
                                if let Some(mpris_event) = player.mpris_event() {
                                    player.mpris_handle(mpris_event);
                                }
                                ctx.request_repaint();
                            }
                            MusicPlayerEvent::PlaybackStarted => {
                                let mut cover = cover.lock();

                                if let Some(track) = player.current_track()
                                    && let Ok(front_cover) = track.read_front_cover()
                                {
                                    if cover.ne(&front_cover) {
                                        *cover = front_cover;

                                        ctx.forget_image(COVER_IMAGE_URI);
                                    }
                                } else if cover.is_some() {
                                    *cover = None;

                                    ctx.forget_image(COVER_IMAGE_URI);
                                }

                                ctx.request_repaint();
                            }
                            MusicPlayerEvent::PlaybackProgress => {
                                player.mpris_update_progress();
                            }
                            MusicPlayerEvent::PlaybackEnded => {
                                let mut playlist = playlist.lock();

                                if let Some(track) = playlist.next_track() {
                                    player.play_track(track.to_owned());
                                }

                                // NOTE: Repaint is needed after doing something with playlist and
                                // player so that the UI state isn't stale.
                                ctx.request_repaint();
                            }
                            _ => {}
                        }
                    }
                }
            });
        }

        Self {
            player,
            playlist,
            cover,
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut player = self.player.lock();
        let mut playlist = self.playlist.lock();

        let frame = egui::frame::Frame::new()
            .fill(ctx.style().visuals.panel_fill)
            .inner_margin(12);

        egui::TopBottomPanel::bottom("controls")
            .frame(frame)
            .show(ctx, |ui| {
                ui.add(ControlPanel::new(&mut player));
                // TODO: Scan progress.
            });

        egui::SidePanel::left("music_metadata")
            .frame(frame)
            .resizable(false)
            .show(ctx, |ui| {
                let mut cover_image =
                    egui::Image::new(egui::include_image!("../assets/album-placeholder.png"));

                if !player.is_empty()
                    && let Some(cover) = self.cover.lock().as_deref()
                {
                    cover_image = egui::Image::from_bytes(COVER_IMAGE_URI, cover.to_owned())
                }

                let mut cursor = ui.cursor();

                cursor.set_width(COVER_IMAGE_SIZE.0);
                cursor.set_height(COVER_IMAGE_SIZE.1);

                ui.painter().rect_filled(
                    cursor,
                    ctx.style().noninteractive().corner_radius,
                    ctx.style().visuals.extreme_bg_color,
                );

                ui.add_sized(COVER_IMAGE_SIZE, cover_image);

                if let Some(current_track) = player.current_track()
                    && !player.is_empty()
                {
                    ui.vertical_centered(|ui| {
                        match (
                            current_track.album.as_deref(),
                            current_track.title.as_deref(),
                        ) {
                            (Some(album), Some(title)) => {
                                ui.heading(format!("{album} - {title}"));
                            }
                            (Some(album), None) => {
                                ui.heading(album);
                            }
                            (None, Some(title)) => {
                                ui.heading(title);
                            }
                            (None, None) => {}
                        }
                    });
                }
            });

        egui::CentralPanel::default().frame(frame).show(ctx, |ui| {
            ui.add(TrackList::new(&mut player, &mut playlist));
        });
    }
}
