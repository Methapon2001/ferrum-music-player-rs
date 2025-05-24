use std::io::Seek;

use eframe::egui;
use lofty::file::TaggedFileExt;
use rodio::Source;

use crate::{common::TrackInfo, ui::controls};

const COVER_IMAGE_URI: &str = "bytes://music_cover";

pub struct App {
    /// `OutputStream` must not be dropped.
    #[allow(dead_code)]
    audio_stream: rodio::OutputStream,
    audio_sink: rodio::Sink,
    track: Option<TrackInfo>,
    track_list: Option<Vec<std::path::PathBuf>>,
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
            });
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical()
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
                                            let mut file = std::fs::File::open(item).unwrap();

                                            let mut track_info = TrackInfo::default();

                                            let tagged_file = lofty::read_from(&mut file).unwrap();
                                            let front_cover =
                                                tagged_file.primary_tag().and_then(|tag| {
                                                    tag.get_picture_type(
                                                        lofty::picture::PictureType::CoverFront,
                                                    )
                                                });

                                            if let Some(cover) = front_cover {
                                                track_info.front_cover =
                                                    Some(cover.data().to_owned());
                                                ctx.forget_image(COVER_IMAGE_URI);
                                            }

                                            if file.seek(std::io::SeekFrom::Start(0)).is_ok() {
                                                let decoded_audio =
                                                    rodio::Decoder::try_from(file).unwrap();

                                                track_info.total_duration =
                                                    decoded_audio.total_duration();

                                                // TODO: Implement your own queue and sink so
                                                // modify source while playing is possible?

                                                self.audio_sink.clear();
                                                self.audio_sink.append(decoded_audio);
                                                self.audio_sink.play();
                                            }

                                            self.track = Some(track_info);
                                        }
                                        ui.label(item.to_str().unwrap().to_owned());
                                    });
                                }
                            } else if let Some(home) = &mut std::env::home_dir() {
                                home.push("Music");
                                // TODO: Read track info and store in sqlite in background.
                                self.track_list = Some(scan_music_files(home).unwrap());
                            }
                        })
                    })
                });
        });
    }
}

fn scan_music_files(dir: &std::path::Path) -> std::io::Result<Vec<std::path::PathBuf>> {
    let mut list: Vec<std::path::PathBuf> = vec![];

    if dir.is_dir() {
        for entry in std::fs::read_dir(dir)? {
            let path = entry?.path();

            if path.is_dir() {
                list.append(&mut scan_music_files(&path)?);
            }

            if let Some("flac" | "wav" | "mp3") = path.extension().map(|v| v.to_str().unwrap()) {
                list.push(path);
            }
        }
    }

    Ok(list)
}
