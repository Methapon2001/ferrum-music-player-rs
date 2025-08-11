use std::{
    sync::{Arc, mpsc},
    thread,
};

use eframe::egui::{self, FontData, FontDefinitions, FontFamily};
use font_kit::{family_name::FamilyName, handle::Handle, source::SystemSource};

use crate::{
    player::MediaPlayer,
    track::{Track, scan_tracks},
    ui::ControlPanel,
};

const COVER_IMAGE_URI: &str = "bytes://music_cover";

pub struct App {
    player: MediaPlayer,
    tracks: Option<Vec<Track>>,
    search: String,
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

        let tracks = if let Some(audio_dir) = &dirs::audio_dir() {
            scan_tracks(audio_dir).ok()
        } else {
            None
        };

        let ctx = cc.egui_ctx.clone();
        let (player_tx, player_rx) = mpsc::sync_channel(0);

        thread::spawn(move || {
            loop {
                let _ = player_rx.recv();

                ctx.request_repaint();
            }
        });

        Self {
            player: MediaPlayer::new(player_tx),
            search: String::new(),
            tracks,
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.player.mpris_handler();

        egui::TopBottomPanel::bottom("controls")
            .show_separator_line(true)
            .show(ctx, |ui| {
                ui.add_space(10.0);

                ui.add(ControlPanel::new(&mut self.player));

                ui.add_space(10.0);

                // TODO: Scan progress.
            });

        egui::SidePanel::left("music_metadata")
            .resizable(false)
            .show(ctx, |ui| {
                let mut cover_image =
                    egui::Image::new(egui::include_image!("../assets/album-placeholder.png"));

                if !self.player.is_empty()
                    && let Some(cover) = self
                        .player
                        .get_track()
                        .as_ref()
                        .and_then(|v| v.cover.as_deref())
                {
                    cover_image = egui::Image::from_bytes(COVER_IMAGE_URI, cover.to_owned())
                        .show_loading_spinner(false);
                }

                ui.add_sized([275.0, 275.0], cover_image);

                if let Some(current_track) = self.player.get_track()
                    && !self.player.is_empty()
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
            egui::ScrollArea::vertical()
                .auto_shrink(false)
                .scroll_source(egui::scroll_area::ScrollSource::ALL)
                .show(ui, |ui| {
                    ui.add_sized(
                        [ui.available_width(), 30.0],
                        egui::TextEdit::singleline(&mut self.search)
                            .vertical_align(egui::Align::Center)
                            .hint_text("Search"),
                    );

                    ui.add_space(10.0);

                    if let Some(list) = &self.tracks {
                        for item in list {
                            let display_text = format!(
                                "{} - {}.{:02} {} / {}",
                                item.album.as_deref().unwrap_or("-"),
                                item.disc.to_owned().unwrap_or(1),
                                item.track.to_owned().unwrap_or(1),
                                item.title.as_deref().unwrap_or("-"),
                                item.artist.as_deref().unwrap_or("-"),
                            );

                            if !self.search.is_empty()
                                && !display_text
                                    .to_lowercase()
                                    .contains(&self.search.to_lowercase())
                            {
                                continue;
                            }

                            ui.horizontal(|ui| {
                                if ui.button("Play").clicked() {
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

                                ui.label(display_text);
                            });
                        }
                    }
                })
        });
    }
}
