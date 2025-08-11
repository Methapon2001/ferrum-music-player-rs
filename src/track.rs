use std::{ffi::OsStr, path::PathBuf, time::Duration};

use lofty::{
    file::{AudioFile, TaggedFileExt},
    tag::Accessor,
};
use walkdir::WalkDir;

#[allow(unused)]
#[derive(Default, Clone, Debug, PartialEq, Eq)]
pub struct Track {
    pub path: PathBuf,
    pub title: Option<String>,
    pub album: Option<String>,
    pub album_artist: Option<String>,
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

        Ok(lofty::read_from_path(path)?.primary_tag().and_then(|tag| {
            tag.get_picture_type(lofty::picture::PictureType::CoverFront)
                .or_else(|| tag.pictures().first())
                .map(|pic| pic.data().to_owned())
        }))
    }
}

/// Scans the given path for music files and reads their metadata.
///
/// This function recursively traverses directories, collecting `TrackInfo` for supported
/// music file types (`.flac`, `.wav`, `.mp3`).
///
/// # Arguments
///
/// * `path` - The starting path to scan. This can be a file or a directory.
///
/// # Returns
///
/// A `Result` which is:
/// - `Ok(Vec<TrackInfo>)` containing a list of `TrackInfo` for all music files found.
/// - `Err(std::io::Error)` if an I/O error occurs during directory traversal.
pub fn scan_tracks(path: &std::path::Path) -> std::io::Result<Vec<Track>> {
    let mut list: Vec<Track> = vec![];

    let walker = WalkDir::new(path)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|entry| {
            entry.file_type().is_file()
                && matches!(
                    entry.path().extension().and_then(OsStr::to_str),
                    Some("flac" | "mp3" | "wav")
                )
        });

    for entry in walker {
        if let Ok(track) = read_track_metadata(entry.path()) {
            list.push(track);
        }
    }

    Ok(list)
}

/// Reads metadata from a music file.
///
/// This function attempts to read metadata from file
/// using the `lofty` crate. It returns a `Result` to indicate whether the
/// operation was successful and an `Option<Track>` to represent if a primary tag
/// was found within the file.
///
/// # Arguments
///
/// * `path` - The path to the music file.
///
/// # Returns
///
/// A `Result<Option<Track>, lofty::error::LoftyError>`:
/// - `Ok(Some(Track))` if the file is a supported music format and a primary tag
///   with metadata was successfully read.
/// - `Ok(None)` if the file is not a supported music format (based on its extension).
/// - `Err(lofty::error::LoftyError)` if an error occurred while reading the music file
///   or its tags
pub fn read_track_metadata(
    path: &std::path::Path,
) -> std::result::Result<Track, lofty::error::LoftyError> {
    let tagged = lofty::read_from_path(path)?;

    Ok(tagged.primary_tag().map_or_else(
        || Track {
            path: path.to_owned(),
            ..Default::default()
        },
        |tag| Track {
            path: path.to_owned(),
            title: tag
                .get_string(&lofty::tag::ItemKey::TrackTitle)
                .map(String::from),
            album: tag
                .get_string(&lofty::tag::ItemKey::AlbumTitle)
                .map(String::from),
            album_artist: tag
                .get_string(&lofty::tag::ItemKey::AlbumArtist)
                .map(String::from),
            artist: tag
                .get_string(&lofty::tag::ItemKey::TrackArtist)
                .map(String::from),
            disc: tag.disk(),
            disc_total: tag.disk_total(),
            track: tag.track(),
            track_total: tag.track_total(),
            duration: Some(tagged.properties().duration()),
            cover: None,
        },
    ))
}
