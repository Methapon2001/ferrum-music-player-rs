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
    track: Option<TrackInfo>,
    track_list: Option<Vec<TrackInfo>>,
}

impl Default for App {
    fn default() -> Self {
        let audio_stream = rodio::OutputStreamBuilder::open_default_stream().unwrap();
        let audio_sink = rodio::Sink::connect_new(audio_stream.mixer());

        Self {
            audio_stream,
            audio_sink,
            track: None,
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

        Self::default()
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::bottom("controls")
            .show_separator_line(true)
            .show(ctx, |ui| {
                ui.add_space(10.0);

                ui.add(controls::Controller::new(&self.audio_sink, &self.track));

                ui.add_space(10.0);

                // TODO: Scan progress.
            });
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink(false)
                .drag_to_scroll(true)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        if !self.audio_sink.empty() {
                            if let Some(cover) =
                                self.track.as_ref().and_then(|t| t.front_cover.clone())
                            {
                                ui.add_sized(
                                    [256.0, 256.0],
                                    egui::Image::from_bytes(COVER_IMAGE_URI, cover),
                                );
                            }
                        }

                        ui.vertical(|ui| {
                            if let Some(list) = &self.track_list {
                                for item in list {
                                    ui.horizontal(|ui| {
                                        if ui.button("Play").clicked() {
                                            let mut file =
                                                std::fs::File::open(item.path.as_ref().unwrap())
                                                    .unwrap();

                                            if item.front_cover.is_some() {
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

                                            self.track = Some(item.to_owned());
                                        }

                                        ui.label(format!(
                                            "{}.{:02} {} - {} / {}",
                                            item.disc.to_owned().unwrap_or(1),
                                            item.track.to_owned().unwrap_or(1),
                                            item.album.to_owned().unwrap_or("-".to_string()),
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

fn scan_music_files(dir: &std::path::Path) -> std::io::Result<Vec<TrackInfo>> {
    let mut list: Vec<TrackInfo> = vec![];

    if dir.is_dir() {
        for entry in std::fs::read_dir(dir)? {
            let path = entry?.path();

            if path.is_dir() {
                list.append(&mut scan_music_files(&path)?);
            }

            if let Some("flac" | "wav" | "mp3") = path.extension().map(|v| v.to_str().unwrap()) {
                let tagged_file = lofty::read_from_path(&path).ok();

                // TODO: Store this in sqlite and only load picture only when select or play track.
                if let Some(info) = tagged_file {
                    let tag = info.primary_tag().unwrap();

                    let track = TrackInfo {
                        // front_cover: tag
                        //     .get_picture_type(lofty::picture::PictureType::CoverFront)
                        //     .map(|v| v.data().to_owned()),
                        front_cover: None,
                        disc: tag.disk(),
                        disc_total: tag.disk_total(),
                        track: tag.track(),
                        track_total: tag.track_total(),
                        album: tag.album().map(|v| v.to_string()),
                        artist: tag.artist().map(|v| v.to_string()),
                        title: tag.title().map(|v| v.to_string()),
                        total_duration: Some(info.properties().duration()),
                        path: Some(path),
                    };

                    list.push(track);
                }
            }
        }
    }

    Ok(list)
}
