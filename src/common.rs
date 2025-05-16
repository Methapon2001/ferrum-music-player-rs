use std::time::Duration;

#[derive(Default)]
pub struct TrackInfo {
    pub front_cover: Option<Vec<u8>>,
    pub total_duration: Option<Duration>,
}
