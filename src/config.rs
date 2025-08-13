use std::path::PathBuf;

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
