use std::{
    sync::{Arc, mpsc},
    thread,
};

use eframe::egui::{self, FontData, FontDefinitions, FontFamily, mutex::Mutex};
use font_kit::{family_name::FamilyName, handle::Handle, source::SystemSource};

use crate::{
    config::COVER_IMAGE_URI,
    database::{Database, get_all_tracks},
    player::{MediaPlayer, MediaPlayerEvent},
    track::Track,
    ui::{control_panel::ControlPanel, track_list::TrackList},
};

pub struct App {
    player: Arc<Mutex<MediaPlayer>>,
    tracks: Arc<Mutex<Vec<Track>>>,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> App {
        egui_extras::install_image_loaders(&cc.egui_ctx);

        let mut font_definitions = FontDefinitions::default();

        for name in ["Noto Sans", "Noto Sans JP", "Noto Sans CJK JP"] {
            let buf = match SystemSource::new().select_best_match(
                &[FamilyName::Title(name.to_string())],
                &font_kit::properties::Properties::new(),
            ) {
                Ok(Handle::Memory { bytes, .. }) => Some(bytes.to_vec()),
                Ok(Handle::Path { path, .. }) => std::fs::read(path).ok(),
                Err(_) => None,
            };

            if let Some(buf) = buf {
                font_definitions
                    .font_data
                    .insert(name.to_owned(), Arc::new(FontData::from_owned(buf)));

                font_definitions
                    .families
                    .get_mut(&FontFamily::Proportional)
                    .unwrap()
                    .insert(0, name.to_owned());

                font_definitions
                    .families
                    .get_mut(&FontFamily::Monospace)
                    .unwrap()
                    .push(name.to_owned());
            }
        }

        cc.egui_ctx.set_fonts(font_definitions);

        // TODO: Make this configurable.
        cc.egui_ctx.options_mut(|options| {
            options.input_options.line_scroll_speed = 100.0;
        });

        let (player_tx, player_rx) = mpsc::sync_channel(0);
        let player = Arc::new(Mutex::new(MediaPlayer::new(player_tx)));

        let tracks = Arc::new(Mutex::new(Vec::new()));

        {
            let tracks = tracks.clone();
            let player = player.clone();
            let ctx = cc.egui_ctx.clone();

            thread::spawn(move || {
                let database = Database::new().expect("Database connected.");

                database.refresh_library(false).ok();

                *tracks.lock() = get_all_tracks(&database.get_connection()).unwrap_or_default();

                ctx.request_repaint();

                // TODO: Handle deleted tracks.

                loop {
                    if let Ok(player_event) = player_rx.recv() {
                        match player_event {
                            MediaPlayerEvent::Tick => {
                                let mut player = player.lock();
                                if let Some(mpris_event) = player.mpris.try_recv_event() {
                                    player.mpris_handle(mpris_event);
                                }
                            }
                        }
                    }
                }
            });
        }

        Self { player, tracks }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut player = self.player.lock();

        player.mpris_update_progress();

        egui::TopBottomPanel::bottom("controls")
            .show_separator_line(true)
            .show(ctx, |ui| {
                ui.add_space(10.0);

                ui.add(ControlPanel::new(&mut player));

                ui.add_space(10.0);

                // TODO: Scan progress.
            });

        egui::SidePanel::left("music_metadata")
            .resizable(false)
            .show(ctx, |ui| {
                let mut cover_image =
                    egui::Image::new(egui::include_image!("../assets/album-placeholder.png"));

                if !player.is_empty()
                    && let Some(cover) =
                        player.get_track().as_ref().and_then(|v| v.cover.as_deref())
                {
                    cover_image = egui::Image::from_bytes(COVER_IMAGE_URI, cover.to_owned())
                        .show_loading_spinner(false);
                }

                ui.add_sized([275.0, 275.0], cover_image);

                if let Some(current_track) = player.get_track()
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

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add(TrackList::new(&mut player, self.tracks.lock().as_slice()));
        });
    }
}
