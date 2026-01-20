use eframe::egui::{self, ImageSource, Vec2};

pub struct CoverArt<'a> {
    source: ImageSource<'a>,
    size: Option<Vec2>,
}

impl<'a> CoverArt<'a> {
    pub fn new(source: impl Into<ImageSource<'a>>) -> Self {
        Self {
            source: source.into(),
            size: None,
        }
    }

    pub fn size(mut self, vec2: Vec2) -> Self {
        self.size = Some(vec2);
        self
    }
}

impl egui::Widget for CoverArt<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let mut cursor = ui.cursor();

        let size = self.size.unwrap_or(ui.available_size());

        cursor.set_width(size.x);
        cursor.set_height(size.y);

        let style = ui.ctx().style();

        ui.painter().rect_filled(
            cursor,
            style.noninteractive().corner_radius,
            style.visuals.extreme_bg_color,
        );

        let image = ui.add_sized(size, egui::Image::new(self.source).shrink_to_fit());

        image.on_hover_text_at_pointer("Cover Image")
    }
}
