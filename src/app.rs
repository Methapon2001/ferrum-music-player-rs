use std::{io::Seek, time::Duration};

use eframe::egui;
use lofty::file::TaggedFileExt;
use rodio::Source;

#[derive(Default)]
struct Track {
    front_cover: Option<Vec<u8>>,
    total_duration: Option<Duration>,
}

pub struct App {
    /// `OutputStream` must not be dropped.
    #[allow(dead_code)]
    audio_stream: rodio::OutputStream,
    audio_sink: rodio::Sink,
    duration: f32,
    volume: f32,
    track: Option<Track>,
}

impl Default for App {
    fn default() -> Self {
        let audio_stream = rodio::OutputStreamBuilder::open_default_stream().unwrap();
        let audio_sink = rodio::Sink::connect_new(audio_stream.mixer());

        Self {
            audio_stream,
            audio_sink,
            duration: 0.0,
            volume: 1.0,
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
                ui.horizontal(|ui| {
                    // TODO: Separate this section as controls UI.

                    let slider_handle = egui::style::HandleShape::Rect { aspect_ratio: 0.5 };

                    // TODO: New layout and use icon.
                    let play_button = ui.add_enabled(
                        self.audio_sink.is_paused() && !self.audio_sink.empty(),
                        egui::Button::new("Play"),
                    );
                    let pause_button = ui.add_enabled(
                        !self.audio_sink.is_paused() && !self.audio_sink.empty(),
                        egui::Button::new("Pause"),
                    );
                    let stop_button =
                        ui.add_enabled(!self.audio_sink.empty(), egui::Button::new("Stop"));

                    if play_button.clicked() {
                        self.audio_sink.play();
                    }
                    if pause_button.clicked() {
                        self.audio_sink.pause();
                    }
                    if stop_button.clicked() {
                        self.audio_sink.clear();
                        self.track = None;
                    }

                    ui.separator();

                    {
                        ui.spacing_mut().slider_width = 75.0;
                        // TODO: Custom?
                        let volume_slider = ui.add(
                            egui::Slider::new(&mut self.volume, 0.0..=1.0)
                                .handle_shape(slider_handle)
                                .show_value(false),
                        );
                        if volume_slider.dragged() {
                            self.audio_sink.set_volume(self.volume);
                        }
                    }

                    ui.separator();

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // NOTE: Default to 1.0 so slider handle will be at the start.
                        let total_duration = if let Some(track) = &self.track {
                            track.total_duration.map(|t| t.as_secs_f32()).unwrap_or(1.0)
                        } else {
                            1.0
                        };

                        self.duration = self.audio_sink.get_pos().as_secs_f32();

                        // TODO: Handle unknown total duration.
                        if !self.audio_sink.empty() {
                            ctx.request_repaint_after(Duration::from_millis(100));
                            ui.label(format!(
                                "{:02}:{:02} / {:02}:{:02}",
                                self.duration.trunc() as u64 / 60,
                                self.duration.trunc() as u64 % 60,
                                total_duration.trunc() as u64 / 60,
                                total_duration.trunc() as u64 % 60
                            ));
                        } else {
                            ui.label("--:-- / --:--");
                        }

                        {
                            ui.spacing_mut().slider_width = ui.available_width();
                            let duration_slider = ui.add_enabled(
                                !self.audio_sink.empty(),
                                egui::Slider::new(&mut self.duration, 0.0..=total_duration)
                                    .handle_shape(slider_handle)
                                    .show_value(false),
                            );
                            if duration_slider.dragged() {
                                self.audio_sink.pause();
                                self.audio_sink
                                    .try_seek(Duration::from_secs_f32(self.duration))
                                    .unwrap();
                            }
                            if duration_slider.drag_stopped() {
                                self.audio_sink.play();
                            }
                        }
                    })
                });
                ui.add_space(10.0);
            });
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                let cover_image_uri = "bytes://music_cover";

                if ui.add(egui::Button::new("Open file…")).clicked() {
                    if let Some(path) = rfd::FileDialog::new().pick_file() {
                        let mut file = std::fs::File::open(path).unwrap();

                        let mut track = Track::default();

                        let tagged_file = lofty::read_from(&mut file).unwrap();
                        let front_cover = tagged_file.primary_tag().and_then(|tag| {
                            tag.get_picture_type(lofty::picture::PictureType::CoverFront)
                        });

                        if let Some(cover) = front_cover {
                            track.front_cover = Some(cover.data().to_owned());
                            ctx.forget_image(cover_image_uri);
                        }

                        if file.seek(std::io::SeekFrom::Start(0)).is_ok() {
                            let decoded_audio = rodio::Decoder::try_from(file).unwrap();

                            track.total_duration = decoded_audio.total_duration();

                            // TODO:
                            //
                            // Implement your own queue and sink so
                            // modify source while playing is possible?

                            self.audio_sink.clear();
                            self.audio_sink.append(decoded_audio);
                            self.audio_sink.play();
                        }

                        self.track = Some(track);
                    }
                }

                if !self.audio_sink.empty() {
                    if let Some(cover) = self.track.as_ref().and_then(|t| t.front_cover.clone()) {
                        ui.add(egui::Image::from_bytes(cover_image_uri, cover));
                    }
                }

                // TODO: Scan music and display as table.
            });
        });
    }
}
