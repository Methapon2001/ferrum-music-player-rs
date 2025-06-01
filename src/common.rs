use std::{path::PathBuf, time::Duration};

use lofty::file::TaggedFileExt;

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

impl TrackInfo {
    pub fn read_front_cover(&self) -> Result<Option<Vec<u8>>, lofty::error::LoftyError> {
        if let Some(path) = self.path.as_ref() {
            if let Some("flac" | "wav" | "mp3") = path.extension().and_then(|v| v.to_str()) {
                Ok(lofty::read_from_path(path)?.primary_tag().and_then(|tag| {
                    tag.get_picture_type(lofty::picture::PictureType::CoverFront)
                        .map(|v| v.data().to_owned())
                }))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }
}
