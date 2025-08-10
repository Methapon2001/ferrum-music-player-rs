mod app;
mod player;
mod track;
mod ui;

use app::App;
use eframe::egui;

fn main() -> eframe::Result {
    eframe::run_native(
        "App",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([800.0, 450.0])
                .with_min_inner_size([650.0, 300.0]),
            ..Default::default()
        },
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
}
