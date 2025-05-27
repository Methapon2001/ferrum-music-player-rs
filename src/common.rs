use std::{path::PathBuf, time::Duration};

#[allow(unused)]
#[derive(Default, Clone, Debug)]
pub struct TrackInfo {
    pub album: Option<String>,
    pub disc: Option<u32>,
    pub disc_total: Option<u32>,
    pub track: Option<u32>,
    pub track_total: Option<u32>,
    pub artist: Option<String>,
    pub title: Option<String>,
    pub path: Option<PathBuf>,
    pub front_cover: Option<Vec<u8>>,
    pub total_duration: Option<Duration>,
}
