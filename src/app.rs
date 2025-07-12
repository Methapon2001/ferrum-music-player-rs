use std::{io::Seek, sync::Arc};

use eframe::egui::{self, FontData, FontDefinitions, FontFamily};
use font_kit::{family_name::FamilyName, handle::Handle, source::SystemSource};
use lofty::{
    file::{AudioFile, TaggedFileExt},
    tag::Accessor,
};

use crate::{common::TrackInfo, ui::controls};

const COVER_IMAGE_URI: &str = "bytes://music_cover";

pub struct App {
    /// `OutputStream` must not be dropped.
    #[allow(dead_code)]
    audio_stream: rodio::OutputStream,
    audio_sink: rodio::Sink,
    track_info: Option<TrackInfo>,
    track_list: Option<Vec<TrackInfo>>,
}

impl Default for App {
    fn default() -> Self {
        let audio_stream = rodio::OutputStreamBuilder::open_default_stream().unwrap();
        let audio_sink = rodio::Sink::connect_new(audio_stream.mixer());

        Self {
            audio_stream,
            audio_sink,
            track_info: None,
            track_list: None,
        }
    }
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

        Self::default()
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::bottom("controls")
            .show_separator_line(true)
            .show(ctx, |ui| {
                ui.add_space(10.0);

                ui.add(controls::Controller::new(
                    &self.audio_sink,
                    &self.track_info,
                ));

                ui.add_space(10.0);

                // TODO: Scan progress.
            });
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink(false)
                .scroll_source(egui::scroll_area::ScrollSource::ALL)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let mut cover_image = egui::Image::new(egui::include_image!(
                            "../assets/album-placeholder.png"
                        ));

                        if !self.audio_sink.empty() {
                            if let Some(cover) =
                                self.track_info.as_ref().and_then(|t| t.front_cover.clone())
                            {
                                cover_image = egui::Image::from_bytes(COVER_IMAGE_URI, cover);
                            }
                        }

                        ui.add_sized([256.0, 256.0], cover_image);

                        ui.vertical(|ui| {
                            if let Some(list) = &self.track_list {
                                for item in list {
                                    ui.horizontal(|ui| {
                                        if ui.button("Play").clicked() {
                                            let mut file =
                                                std::fs::File::open(item.path.as_ref().unwrap())
                                                    .unwrap();

                                            let mut track = item.to_owned();

                                            if let Ok(front_cover) = track.read_front_cover() {
                                                track.front_cover = front_cover;
                                            }

                                            if track.front_cover.is_some() {
                                                ctx.forget_image(COVER_IMAGE_URI);
                                            }

                                            if file.seek(std::io::SeekFrom::Start(0)).is_ok() {
                                                let decoded_audio =
                                                    rodio::Decoder::try_from(file).unwrap();

                                                // TODO: Implement your own queue and sink so
                                                // modify source while playing is possible?

                                                self.audio_sink.clear();
                                                self.audio_sink.append(decoded_audio);
                                                self.audio_sink.play();
                                            }

                                            self.track_info = Some(track);
                                        }

                                        ui.label(format!(
                                            "{} - {}.{:02} {} / {}",
                                            item.album.to_owned().unwrap_or("-".to_string()),
                                            item.disc.to_owned().unwrap_or(1),
                                            item.track.to_owned().unwrap_or(1),
                                            item.title.to_owned().unwrap_or("-".to_string()),
                                            item.artist.to_owned().unwrap_or("-".to_string()),
                                        ));
                                    });
                                }
                            } else if let Some(home) = &mut std::env::home_dir() {
                                home.push("Music");
                                // TODO: Scan and read track info then store in sqlite in background.
                                self.track_list = scan_music_files(home).ok();
                            }
                        })
                    })
                });
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
fn scan_music_files(path: &std::path::Path) -> std::io::Result<Vec<TrackInfo>> {
    let mut list: Vec<TrackInfo> = vec![];

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
) -> std::result::Result<Option<TrackInfo>, lofty::error::LoftyError> {
    if let Some("flac" | "wav" | "mp3") = path.extension().and_then(|v| v.to_str()) {
        // TODO: Store this in sqlite and only load picture only when select or play track.
        let tagged = lofty::read_from_path(path)?;

        Ok(tagged.primary_tag().map(|tag| TrackInfo {
            front_cover: None,
            disc: tag.disk(),
            disc_total: tag.disk_total(),
            track: tag.track(),
            track_total: tag.track_total(),
            album: tag.album().map(|v| v.to_string()),
            artist: tag.artist().map(|v| v.to_string()),
            title: tag.title().map(|v| v.to_string()),
            total_duration: Some(tagged.properties().duration()),
            path: Some(path.to_owned()),
        }))
    } else {
        Ok(None)
    }
}
