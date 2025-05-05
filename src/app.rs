use eframe::egui;

#[derive(Default)]
pub struct App {}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.add(egui::Label::new(
                    egui::RichText::new("EGUI Music Player").heading(),
                ));

                if ui.add(egui::Button::new("Open file…")).clicked() {
                    if let Some(path) = rfd::FileDialog::new().pick_file() {
                        ui.label(path.as_path().to_str().unwrap());
                    }
                }
            });
        });
    }
}
