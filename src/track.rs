use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
    result::Result,
    time::{Duration, SystemTime},
};

use chrono::{DateTime, Local};
use lofty::{
    config::ParseOptions,
    error::LoftyError,
    file::{AudioFile, TaggedFileExt},
    picture::PictureType,
    probe::Probe,
    tag::ItemKey,
};
use souvlaki::MediaMetadata;
use walkdir::WalkDir;

#[allow(unused)]
#[derive(Default, Clone, Debug, PartialEq, Eq)]
pub struct Track {
    pub path: PathBuf,
    pub modified: Option<String>,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub genre: Option<String>,
    pub album: Option<String>,
    pub album_artist: Option<String>,
    pub duration: Option<Duration>,
    pub disc: Option<String>,
    pub disc_total: Option<String>,
    pub track: Option<String>,
    pub track_total: Option<String>,
    pub cover: Option<Vec<u8>>,
}

impl Track {
    pub fn read_front_cover(&self) -> Result<Option<Vec<u8>>, LoftyError> {
        let path = self.path.as_path();

        Ok(lofty::read_from_path(path)?.primary_tag().and_then(|tag| {
            tag.get_picture_type(PictureType::CoverFront)
                .or_else(|| tag.pictures().first())
                .map(|pic| pic.data().to_owned())
        }))
    }
}

impl AsRef<Track> for Track {
    fn as_ref(&self) -> &Track {
        self
    }
}

impl<'a> From<&'a Track> for MediaMetadata<'a> {
    fn from(val: &'a Track) -> Self {
        MediaMetadata {
            album: val.album.as_deref(),
            title: val.title.as_deref(),
            artist: val.artist.as_deref(),
            duration: val.duration,
            cover_url: None,
        }
    }
}

/// Scans the given path for music files.
///
/// This function recursively traverses directories, collecting `Track` for supported
/// music file types (Currently plan to support `.flac`, `.wav`, `.mp3`).
///
/// # Arguments
///
/// * `path` - The starting path to scan. This can be a file or a directory.
///
/// # Returns
///
/// A `Result` which is:
/// - `Vec<PathBuf>` containing a list of `PathBuf` for all music files found.
pub fn scan_tracks(path: &Path) -> Vec<PathBuf> {
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
    walker.map(|entry| entry.path().to_owned()).collect()
}

/// Reads metadata from a music file.
///
/// This function attempts to read metadata from file
/// using the `lofty` crate.
///
/// # Arguments
///
/// * `path` - The path to the music file.
///
/// # Returns
///
/// A `Result<Option<Track>, LoftyError>`:
/// - `Ok(Track)` if the file can be read by lofty.
/// - `Err(LoftyError)` if an error occurred while reading the music file or tags
pub fn read_track_metadata(path: &Path) -> Result<Track, LoftyError> {
    let probe = Probe::open(path)?.options(
        ParseOptions::default()
            .implicit_conversions(false)
            .read_cover_art(false),
    );
    let tagged = probe.read()?;

    Ok(tagged.primary_tag().map_or_else(
        || Track {
            path: path.to_owned(),
            ..Default::default()
        },
        |tag| Track {
            path: path.to_owned(),
            modified: Some(
                DateTime::<Local>::from(
                    path.metadata()
                        .and_then(|m| m.modified())
                        .unwrap_or(SystemTime::now()),
                )
                .to_rfc3339(),
            ),
            title: tag.get_string(ItemKey::TrackTitle).map(String::from),
            artist: tag.get_string(ItemKey::TrackArtist).map(String::from),
            genre: tag.get_string(ItemKey::Genre).map(String::from),
            album: tag.get_string(ItemKey::AlbumTitle).map(String::from),
            album_artist: tag.get_string(ItemKey::AlbumArtist).map(String::from),
            disc: tag.get_string(ItemKey::DiscNumber).map(String::from),
            disc_total: tag.get_string(ItemKey::DiscTotal).map(String::from),
            track: tag.get_string(ItemKey::TrackNumber).map(String::from),
            track_total: tag.get_string(ItemKey::TrackTotal).map(String::from),
            duration: Some(tagged.properties().duration()),
            cover: None,
        },
    ))
}
