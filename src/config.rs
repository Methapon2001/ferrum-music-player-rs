use std::{path::PathBuf, sync::Arc};

use eframe::egui::{FontData, FontDefinitions, FontFamily};

pub const COVER_IMAGE_SIZE: (f32, f32) = (275., 275.);

pub fn get_font_definitions() -> FontDefinitions {
    let mut font_definitions = FontDefinitions::default();

    for name in ["Noto Sans", "Noto Sans JP", "Noto Sans CJK JP"] {
        let buf = match font_kit::source::SystemSource::new().select_best_match(
            &[font_kit::family_name::FamilyName::Title(name.to_string())],
            &font_kit::properties::Properties::new(),
        ) {
            Ok(font_kit::handle::Handle::Memory { bytes, .. }) => Some(bytes.to_vec()),
            Ok(font_kit::handle::Handle::Path { path, .. }) => std::fs::read(path).ok(),
            Err(_) => None,
        };

        if let Some(buf) = buf {
            font_definitions
                .font_data
                .insert(name.to_owned(), Arc::new(FontData::from_owned(buf)));

            if let Some(val) = font_definitions.families.get_mut(&FontFamily::Proportional) {
                val.insert(0, name.to_owned());
            }

            if let Some(val) = font_definitions.families.get_mut(&FontFamily::Monospace) {
                val.insert(0, name.to_owned());
            }
        }
    }

    font_definitions
}

pub fn get_default_app_dir_config() -> PathBuf {
    let mut config_dir = dirs::config_dir().expect("Os config directory.");

    config_dir.push("org.ferrum.Player");

    if !config_dir.exists() {
        std::fs::create_dir(&config_dir).expect("Config directory created.");
    }

    config_dir
}

pub fn get_default_audio_dir_config() -> Option<PathBuf> {
    dirs::audio_dir()
}
