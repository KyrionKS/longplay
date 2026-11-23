//! Library data model. Serialized to the webview in camelCase.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Album {
    pub id: String,
    pub title: String,
    pub album_artist: String,
    pub year: Option<u16>,
    pub genre: Option<String>,
    pub label: Option<String>,
    /// Unix timestamp (seconds) of the newest file in the album.
    pub added_at: i64,
    /// Absolute path of the cached cover, if any.
    pub cover: Option<String>,
    pub palette: Palette,
    pub format: DiscFormat,
    /// Whether the album comes from a release folder or an M3U playlist.
    #[serde(default)]
    pub source: AlbumSource,
    pub sides: Vec<Side>,
    pub folder: String,
    pub track_count: u32,
    pub duration: f64,
}

impl Album {
    pub fn total_tracks(&self) -> u32 {
        self.sides.iter().map(|s| s.tracks.len() as u32).sum()
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DiscFormat {
    Lp12,
    Single7,
    Unknown,
}

/// Where an album comes from. Playlists are pressed as white-label test
/// pressings in the UI; regular folders are releases.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AlbumSource {
    #[default]
    Release,
    Playlist,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Side {
    pub label: String,
    pub rpm: Rpm,
    pub matrix: Option<String>,
    pub tracks: Vec<Track>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Rpm {
    #[serde(rename = "33")]
    R33,
    #[serde(rename = "45")]
    R45,
}

impl Rpm {
    pub fn value(self) -> f64 {
        match self {
            Self::R33 => 100.0 / 3.0,
            Self::R45 => 45.0,
        }
    }

    /// Playback ratio against a 33⅓ reference.
    pub fn ratio(self) -> f64 {
        self.value() / (100.0 / 3.0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    pub id: String,
    pub number: u32,
    pub title: String,
    pub artist: String,
    pub duration: f64,
    pub pregap: f64,
    /// Absolute path of the audio file.
    pub path: String,
    /// Start offset inside the file (CUE single-file albums).
    pub offset: f64,
    pub side: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Palette {
    pub vinyl: String,
    pub vinyl_light: String,
    pub label: String,
    pub accent: String,
    pub dark: String,
}

impl Palette {
    pub fn fallback() -> Self {
        Self {
            vinyl: "#2f5d63".to_string(),
            vinyl_light: "#4d8c93".to_string(),
            label: "#efe6cf".to_string(),
            accent: "#c9a227".to_string(),
            dark: "#1b100a".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rpm_ratio_is_one_for_33() {
        assert!((Rpm::R33.ratio() - 1.0).abs() < 1e-9);
        assert!((Rpm::R45.ratio() - 1.35).abs() < 1e-9);
    }

    #[test]
    fn serializes_rpm_as_number_string() {
        let json = serde_json::to_string(&Rpm::R45).unwrap();
        assert_eq!(json, "\"45\"");
    }

    #[test]
    fn serializes_album_in_camel_case() {
        let album = Album {
            id: "a".into(),
            title: "T".into(),
            album_artist: "A".into(),
            year: None,
            genre: None,
            label: None,
            added_at: 0,
            cover: None,
            palette: Palette::fallback(),
            format: DiscFormat::Lp12,
            source: AlbumSource::Release,
            sides: vec![],
            folder: "/m".into(),
            track_count: 0,
            duration: 0.0,
        };
        let json = serde_json::to_string(&album).unwrap();
        assert!(json.contains("\"albumArtist\""));
        assert!(json.contains("\"addedAt\""));
        assert!(json.contains("\"lp12\""));
        assert!(json.contains("\"release\""));
    }

    #[test]
    fn missing_source_defaults_to_release() {
        let mut value = serde_json::to_value(Album {
            id: "a".into(),
            title: "T".into(),
            album_artist: "A".into(),
            year: None,
            genre: None,
            label: None,
            added_at: 0,
            cover: None,
            palette: Palette::fallback(),
            format: DiscFormat::Lp12,
            source: AlbumSource::Playlist,
            sides: vec![],
            folder: "/m".into(),
            track_count: 0,
            duration: 0.0,
        })
        .unwrap();
        value.as_object_mut().unwrap().remove("source");
        let album: Album = serde_json::from_value(value).unwrap();
        assert_eq!(album.source, AlbumSource::Release);
    }
}
