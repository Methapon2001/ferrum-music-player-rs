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
}

impl Default for App {
    fn default() -> Self {
        let audio_stream = rodio::OutputStreamBuilder::open_default_stream().unwrap();
        let audio_sink = rodio::Sink::connect_new(audio_stream.mixer());

        Self {
            audio_stream,
            audio_sink,
            track: None,
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
            egui::ScrollArea::vertical().show(ui, |ui| {
                if ui.add(egui::Button::new("Open file…")).clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("music", &["flac", "wav", "mp3"])
                        .pick_file()
                    {
                        let mut file = std::fs::File::open(path).unwrap();

                        let mut track_info = TrackInfo::default();

                        let tagged_file = lofty::read_from(&mut file).unwrap();
                        let front_cover = tagged_file.primary_tag().and_then(|tag| {
                            tag.get_picture_type(lofty::picture::PictureType::CoverFront)
                        });

                        if let Some(cover) = front_cover {
                            track_info.front_cover = Some(cover.data().to_owned());
                            ctx.forget_image(COVER_IMAGE_URI);
                        }

                        if file.seek(std::io::SeekFrom::Start(0)).is_ok() {
                            let decoded_audio = rodio::Decoder::try_from(file).unwrap();

                            track_info.total_duration = decoded_audio.total_duration();

                            // TODO:
                            //
                            // Implement your own queue and sink so
                            // modify source while playing is possible?

                            self.audio_sink.clear();
                            self.audio_sink.append(decoded_audio);
                            self.audio_sink.play();
                        }

                        self.track = Some(track_info);
                    }
                }

                if !self.audio_sink.empty() {
                    if let Some(cover) = self.track.as_ref().and_then(|t| t.front_cover.clone()) {
                        ui.add(egui::Image::from_bytes(COVER_IMAGE_URI, cover));
                    }
                }

                // TODO: Scan music and display as table.
            });
        });
    }
}
