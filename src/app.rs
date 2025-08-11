use std::{
    sync::{Arc, mpsc},
    thread,
};

use eframe::egui::{self, FontData, FontDefinitions, FontFamily};
use font_kit::{family_name::FamilyName, handle::Handle, source::SystemSource};
use lofty::{
    file::{AudioFile, TaggedFileExt},
    tag::Accessor,
};

use crate::{player::MediaPlayer, track::Track, ui};

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

        let tracks = if let Some(home) = &mut std::env::home_dir() {
            scan_music_files(&home.join("Music")).ok()
        } else {
            None
        };

        let ctx = cc.egui_ctx.clone();
        let (player_tx, player_rx) = mpsc::sync_channel(0);

        thread::spawn(move || {
            loop {
                let _ = player_rx.recv();

                // TODO: Handle event(s)?
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

                ui.add(ui::media::ControlPanel::new(&mut self.player));

                ui.add_space(10.0);

                // TODO: Scan progress.
            });

        egui::SidePanel::left("music_metadata")
            .resizable(false)
            .show(ctx, |ui| {
                let mut cover_image =
                    egui::Image::new(egui::include_image!("../assets/album-placeholder.png"));

                if !self.player.is_empty() {
                    if let Some(cover) = self
                        .player
                        .get_track()
                        .as_ref()
                        .and_then(|t| t.cover.clone())
                    {
                        cover_image = egui::Image::from_bytes(COVER_IMAGE_URI, cover)
                            .show_loading_spinner(false);
                    }
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

/// Scans the given path for music files and reads their metadata.
///
/// This function recursively traverses directories, collecting `TrackInfo` for supported
/// music file types (`.flac`, `.wav`, `.mp3`).
///
/// # Arguments
///
/// * `path` - The starting path to scan. This can be a file or a directory.
///
/// # Returns
///
/// A `Result` which is:
/// - `Ok(Vec<TrackInfo>)` containing a list of `TrackInfo` for all music files found.
/// - `Err(std::io::Error)` if an I/O error occurs during directory traversal.
fn scan_music_files(path: &std::path::Path) -> std::io::Result<Vec<Track>> {
    let mut list: Vec<Track> = vec![];

    if path.is_dir() {
        for entry in std::fs::read_dir(path)? {
            let path = entry?.path();

            if path.is_dir() {
                list.append(&mut scan_music_files(&path)?);
            }

            if let Some(track_info) = read_music_file(&path).unwrap_or(None) {
                list.push(track_info);
            }
        }
    }

    if let Some(track_info) = read_music_file(path).unwrap_or(None) {
        list.push(track_info);
    }

    // TODO: Handle error and display error.

    Ok(list)
}

/// Reads metadata from a single music file.
///
/// This function attempts to read metadata from files with `.flac`, `.wav`, or `.mp3`
/// extensions using the `lofty` crate. It returns a `Result` to indicate whether the
/// operation was successful and an `Option<TrackInfo>` to represent if a primary tag
/// was found within the file.
///
/// # Arguments
///
/// * `path` - The path to the music file.
///
/// # Returns
///
/// A `Result<Option<TrackInfo>, lofty::error::LoftyError>`:
/// - `Ok(Some(TrackInfo))` if the file is a supported music format and a primary tag
///   with metadata was successfully read.
/// - `Ok(None)` if the file is not a supported music format (based on its extension).
/// - `Err(lofty::error::LoftyError)` if an error occurred while reading the music file
///   or its tags
fn read_music_file(
    path: &std::path::Path,
) -> std::result::Result<Option<Track>, lofty::error::LoftyError> {
    if let Some("flac" | "wav" | "mp3") = path.extension().and_then(|v| v.to_str()) {
        // TODO: Store this in sqlite and only load picture only when select or play track.
        let tagged = lofty::read_from_path(path)?;

        Ok(tagged.primary_tag().map(|tag| Track {
            path: path.to_owned(),
            album: tag.album().map(|v| v.to_string()),
            title: tag.title().map(|v| v.to_string()),
            artist: tag.artist().map(|v| v.to_string()),
            disc: tag.disk(),
            disc_total: tag.disk_total(),
            track: tag.track(),
            track_total: tag.track_total(),
            duration: Some(tagged.properties().duration()),
            cover: None,
        }))
    } else {
        Ok(None)
    }
}
