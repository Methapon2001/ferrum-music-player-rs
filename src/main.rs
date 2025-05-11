mod app;

use app::App;
use eframe::egui;

fn main() -> eframe::Result {
    eframe::run_native(
        "App",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([800.0, 540.0])
                .with_min_inner_size([600.0, 400.0]),
            ..Default::default()
        },
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
}
