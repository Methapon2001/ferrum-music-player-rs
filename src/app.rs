use std::sync::{Arc, mpsc};
use std::thread;

use eframe::egui;
use eframe::egui::TextureHandle;
use parking_lot::Mutex;

use crate::{
    config::{COVER_IMAGE_SIZE, get_font_definitions},
    database::{Database, get_all_tracks},
    player::{MusicPlayer, MusicPlayerEvent},
    playlist::Playlist,
    ui::{control_panel::ControlPanel, track_list::TrackList},
};

pub struct App {
    player: Arc<Mutex<MusicPlayer>>,
    cover: Arc<Mutex<Option<TextureHandle>>>,
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
        let cover = Arc::new(Mutex::new(None));

        {
            let player = player.clone();
            let cover = cover.clone();
            let ctx = cc.egui_ctx.clone();

            thread::spawn(move || {
                let database = Database::new().expect("Database connected.");

                database.refresh_library(false).ok();

                // NOTE: Default playlist is the library.
                // TODO: Separate library and playlist.
                *player.lock().playlist_mut() =
                    Playlist::new(get_all_tracks(&database.get_connection()).unwrap_or_default());

                ctx.request_repaint();

                // TODO: Load playlist(s).

                loop {
                    if let Ok(player_event) = player_rx.recv() {
                        match player_event {
                            MusicPlayerEvent::Tick => {
                                let mut player = player.lock();
                                if let Some(mpris_event) = player.mpris_event() {
                                    player.mpris_handle(mpris_event);
                                }
                                ctx.request_repaint();
                            }
                            MusicPlayerEvent::PlaybackStarted => {
                                let track = player.lock().current_track().cloned();

                                let texture = track.and_then(|t| match t.read_front_cover() {
                                    Ok(front_cover) => front_cover.as_deref().and_then(|buffer| {
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
                                    }),
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
                            _ => {}
                        }
                    }
                }
            });
        }

        Self { player, cover }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut player = self.player.lock();

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

                if !player.is_stopped()
                    && let Some(cover) = self.cover.lock().as_ref()
                {
                    cover_image = egui::Image::from_texture(cover);
                }

                let mut cursor = ui.cursor();

                cursor.set_width(COVER_IMAGE_SIZE.0);
                cursor.set_height(COVER_IMAGE_SIZE.1);

                ui.painter().rect_filled(
                    cursor,
                    ctx.style().noninteractive().corner_radius,
                    ctx.style().visuals.extreme_bg_color,
                );

                ui.add_sized(COVER_IMAGE_SIZE, cover_image.shrink_to_fit());

                if let Some(current_track) = player.current_track()
                    && !player.is_stopped()
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
            ui.add(TrackList::new(&mut player));
        });
    }
}
