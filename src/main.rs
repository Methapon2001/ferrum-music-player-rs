use eframe::egui;

fn main() -> eframe::Result {
    env_logger::init();

    eframe::run_native(
        "Ferrum Player",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([800.0, 450.0])
                .with_min_inner_size([650.0, 300.0])
                .with_app_id("org.ferrum.Player"),
            ..Default::default()
        },
        Box::new(|cc| Ok(Box::new(ferrum_music_player::App::new(cc)))),
    )
}
