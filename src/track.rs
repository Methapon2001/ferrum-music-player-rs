use std::{path::PathBuf, time::Duration};

use lofty::file::TaggedFileExt;

#[allow(unused)]
#[derive(Default, Clone, Debug, PartialEq, Eq)]
pub struct Track {
    pub path: PathBuf,
    pub album: Option<String>,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub duration: Option<Duration>,
    pub disc: Option<u32>,
    pub disc_total: Option<u32>,
    pub track: Option<u32>,
    pub track_total: Option<u32>,
    pub cover: Option<Vec<u8>>,
}

impl Track {
    pub fn read_front_cover(&self) -> Result<Option<Vec<u8>>, lofty::error::LoftyError> {
        let path = self.path.as_path();
        if let Some("flac" | "wav" | "mp3") = path.extension().and_then(|v| v.to_str()) {
            Ok(lofty::read_from_path(path)?.primary_tag().and_then(|tag| {
                tag.get_picture_type(lofty::picture::PictureType::CoverFront)
                    .or_else(|| tag.pictures().first())
                    .map(|pic| pic.data().to_owned())
            }))
        } else {
            Ok(None)
        }
    }
}
