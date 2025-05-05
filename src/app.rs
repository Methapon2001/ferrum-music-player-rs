use std::io::Seek;

use eframe::egui;
use lofty::file::TaggedFileExt;

pub struct App {
    /// `OutputStream` must not be dropped.
    #[allow(dead_code)]
    audio_stream: rodio::OutputStream,
    audio_sink: rodio::Sink,
    music_cover: Option<Vec<u8>>,
}

impl Default for App {
    fn default() -> Self {
        let audio_stream = rodio::OutputStreamBuilder::open_default_stream().unwrap();
        let audio_sink = rodio::Sink::connect_new(audio_stream.mixer());

        Self {
            audio_stream,
            audio_sink,
            music_cover: None,
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                let cover_image_uri = "bytes://music_cover";

                ui.add(egui::Label::new(
                    egui::RichText::new("EGUI Music Player").heading(),
                ));

                ui.add_space(50.0);

                if ui.add(egui::Button::new("Open file…")).clicked() {
                    if let Some(path) = rfd::FileDialog::new().pick_file() {
                        let mut file = std::fs::File::open(path).unwrap();

                        let tagged_file = lofty::read_from(&mut file).unwrap();
                        let cover = tagged_file.primary_tag().and_then(|tag| {
                            let count = tag.picture_count();

                            if count > 0 {
                                tag.pictures().first()
                            } else {
                                None
                            }
                        });

                        if let Some(cover) = cover {
                            self.music_cover = Some(cover.data().to_owned());
                            ctx.forget_image(cover_image_uri);
                        }

                        if file.seek(std::io::SeekFrom::Start(0)).is_ok() {
                            self.audio_sink.clear();
                            self.audio_sink
                                .append(rodio::Decoder::try_from(file).unwrap());
                            self.audio_sink.play();
                        }
                    }
                }

                if let Some(cover) = &self.music_cover {
                    ui.add(egui::Image::from_bytes(cover_image_uri, cover.clone()));
                }
            });
        });
    }
}
