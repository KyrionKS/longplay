//! Folder scanning, album grouping and side detection.

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Instant, UNIX_EPOCH};

use walkdir::WalkDir;

use super::artwork;
use super::cue;
use super::db::Db;
use super::metadata;
use super::model::{Album, DiscFormat, Palette, Rpm, Side, Track};
use super::palette;

const AUDIO_EXTENSIONS: &[&str] = &[
    "flac", "mp3", "m4a", "aac", "wav", "aiff", "aif", "ogg", "opus", "alac",
];

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanSummary {
    pub albums: usize,
    pub tracks: usize,
    pub issues: Vec<String>,
    pub elapsed_ms: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum ScanError {
    #[error(transparent)]
    Db(#[from] super::db::DbError),
}

#[derive(Debug, Clone)]
struct BuiltTrack {
    number: u32,
    title: String,
    artist: String,
    duration: f64,
    pregap: f64,
    path: PathBuf,
    offset: f64,
    side_hint: Option<char>,
}

impl BuiltTrack {
    fn span(&self) -> f64 {
        self.duration + self.pregap
    }
}

struct AlbumMeta {
    title: String,
    album_artist: String,
    year: Option<u16>,
    genre: Option<String>,
    label: Option<String>,
    folder: PathBuf,
    added_at: i64,
}

struct CueAlbum {
    album: Album,
    files: Vec<PathBuf>,
    warnings: Vec<String>,
}

pub fn scan(roots: &[PathBuf], db: &mut Db, covers: &Path) -> Result<ScanSummary, ScanError> {
    let started = Instant::now();
    let mut issues = Vec::new();
    let mut albums: Vec<Album> = Vec::new();

    let mut audio_files = Vec::new();
    let mut cue_files = Vec::new();
    collect_files(roots, &mut audio_files, &mut cue_files);

    let mut consumed: HashSet<PathBuf> = HashSet::new();

    for cue_path in &cue_files {
        match build_from_cue(cue_path, covers) {
            Ok(built) => {
                issues.extend(built.warnings);
                consumed.extend(built.files);
                albums.push(built.album);
            }
            Err(issue) => issues.push(issue),
        }
    }

    let mut groups: BTreeMap<String, Vec<(PathBuf, metadata::FileTags)>> = BTreeMap::new();
    for path in audio_files {
        if consumed.contains(&path) {
            continue;
        }
        match metadata::read(&path) {
            Ok(tags) => {
                let key = group_key(&path, &tags);
                groups.entry(key).or_default().push((path, tags));
            }
            Err(error) => issues.push(error.to_string()),
        }
    }

    for entries in groups.into_values() {
        albums.push(build_from_files(entries, covers));
    }

    albums.sort_by_key(|album| std::cmp::Reverse(album.added_at));

    let tracks: usize = albums.iter().map(|album| album.track_count as usize).sum();
    db.replace_albums(&albums)?;

    Ok(ScanSummary {
        albums: albums.len(),
        tracks,
        issues,
        elapsed_ms: started.elapsed().as_millis() as u64,
    })
}

fn collect_files(roots: &[PathBuf], audio: &mut Vec<PathBuf>, cues: &mut Vec<PathBuf>) {
    for root in roots {
        if !root.is_dir() {
            continue;
        }
        for entry in WalkDir::new(root)
            .follow_links(false)
            .into_iter()
            .filter_entry(|entry| !entry.file_name().to_string_lossy().starts_with('.'))
        {
            let Ok(entry) = entry else { continue };
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.into_path();
            let extension = path
                .extension()
                .map(|e| e.to_string_lossy().to_ascii_lowercase())
                .unwrap_or_default();
            if extension == "cue" {
                cues.push(path);
            } else if AUDIO_EXTENSIONS.contains(&extension.as_str()) {
                audio.push(path);
            }
        }
    }
}

fn group_key(path: &Path, tags: &metadata::FileTags) -> String {
    let artist = tags.album_artist.as_ref().or(tags.artist.as_ref());
    match (&tags.album, artist) {
        (Some(album), Some(artist)) if !album.trim().is_empty() && !artist.trim().is_empty() => {
            format!(
                "{}|{}",
                artist.trim().to_lowercase(),
                album.trim().to_lowercase()
            )
        }
        _ => path
            .parent()
            .map(|parent| parent.to_string_lossy().to_lowercase())
            .unwrap_or_default(),
    }
}

fn build_from_files(entries: Vec<(PathBuf, metadata::FileTags)>, covers: &Path) -> Album {
    let mut entries = entries;
    entries.sort_by(|a, b| {
        let key_a = (
            a.1.disc.unwrap_or(1),
            a.1.track.unwrap_or(0),
            a.0.file_name().map(|n| n.to_owned()),
        );
        let key_b = (
            b.1.disc.unwrap_or(1),
            b.1.track.unwrap_or(0),
            b.0.file_name().map(|n| n.to_owned()),
        );
        key_a.cmp(&key_b)
    });

    let first = &entries[0].1;
    let folder = entries[0]
        .0
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_default();
    let folder_name = folder
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "Unbekanntes Album".to_string());

    let mut album_artist = first.album_artist.clone().or_else(|| first.artist.clone());

    let mut album_title = first.album.clone().filter(|title| !title.trim().is_empty());

    // Untagged folders often carry "Artist - Album" in their name.
    if (album_artist.is_none() || album_title.is_none())
        && let Some((folder_artist, folder_album)) = split_folder_name(&folder_name)
    {
        album_artist = album_artist.or(Some(folder_artist));
        album_title = album_title.or(Some(folder_album));
    }

    let album_artist = album_artist.unwrap_or_else(|| "Unbekannter Künstler".to_string());

    let distinct_discs: BTreeSet<u32> = entries.iter().filter_map(|(_, tags)| tags.disc).collect();
    let use_discs = distinct_discs.len() > 1;

    let mut tracks = Vec::with_capacity(entries.len());
    for (index, (path, tags)) in entries.iter().enumerate() {
        let title = tags
            .title
            .clone()
            .or_else(|| {
                path.file_stem()
                    .map(|stem| stem.to_string_lossy().into_owned())
            })
            .unwrap_or_else(|| format!("Track {}", index + 1));

        let side_hint = side_from_title(&title).or_else(|| {
            if use_discs {
                disc_side(tags.disc.unwrap_or(1))
            } else {
                None
            }
        });

        tracks.push(BuiltTrack {
            number: tags.track.unwrap_or(index as u32 + 1),
            title,
            artist: tags.artist.clone().unwrap_or_else(|| album_artist.clone()),
            duration: tags.duration,
            pregap: 0.0,
            path: path.clone(),
            offset: 0.0,
            side_hint,
        });
    }

    let cover_bytes = entries.iter().find_map(|(_, tags)| tags.picture.clone());
    let folder_cover = artwork::find_folder_cover(&folder);
    let cover_source = artwork::bytes(cover_bytes.as_deref(), folder_cover.as_deref());
    let added_at = entries
        .iter()
        .map(|(path, _)| mtime_secs(path))
        .max()
        .unwrap_or_else(super::db::now_secs);

    assemble(
        AlbumMeta {
            title: album_title.unwrap_or(folder_name),
            album_artist,
            year: first.year,
            genre: first.genre.clone(),
            label: first.label.clone(),
            folder,
            added_at,
        },
        tracks,
        cover_source,
        covers,
    )
}

fn build_from_cue(cue_path: &Path, covers: &Path) -> Result<CueAlbum, String> {
    let bytes = fs::read(cue_path).map_err(|e| format!("{}: {e}", cue_path.display()))?;
    let sheet = cue::parse_bytes(&bytes).map_err(|e| format!("{}: {e}", cue_path.display()))?;
    let folder = cue_path.parent().map(Path::to_path_buf).unwrap_or_default();

    let files: Vec<Option<PathBuf>> = sheet
        .files
        .iter()
        .map(|file| {
            let path = folder.join(&file.name);
            path.is_file().then_some(path)
        })
        .collect();

    if files.iter().all(Option::is_none) {
        return Err(format!(
            "{}: keine der referenzierten Audiodateien gefunden",
            cue_path.display()
        ));
    }

    let durations: Vec<f64> = files
        .iter()
        .map(|path| {
            path.as_ref()
                .and_then(|p| metadata::duration(p).ok())
                .unwrap_or(0.0)
        })
        .collect();

    let album_artist = sheet
        .performer
        .clone()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "Unbekannter Künstler".to_string());

    let mut tracks = Vec::new();
    for (index, track) in sheet.tracks.iter().enumerate() {
        let Some(start) = track.start() else {
            continue;
        };
        let file_index = match track.file_index {
            Some(index) => index,
            None if sheet.files.len() == 1 => 0,
            None => continue,
        };
        let Some(file_path) = files.get(file_index).and_then(|p| p.clone()) else {
            continue;
        };

        let file_duration = durations.get(file_index).copied().unwrap_or(0.0);
        let end = sheet
            .tracks
            .iter()
            .skip(index + 1)
            .filter(|next| next.file_index == Some(file_index))
            .find_map(|next| next.start().filter(|next_start| *next_start > start))
            .unwrap_or(file_duration);
        let duration = (end - start).max(0.1);

        let title = track
            .title
            .clone()
            .or_else(|| {
                file_path
                    .file_stem()
                    .map(|stem| stem.to_string_lossy().into_owned())
            })
            .unwrap_or_else(|| format!("Track {}", index + 1));

        tracks.push(BuiltTrack {
            number: track.number,
            title: title.clone(),
            artist: track
                .performer
                .clone()
                .unwrap_or_else(|| album_artist.clone()),
            duration,
            pregap: track.pregap(),
            path: file_path,
            offset: start,
            side_hint: track.side.or_else(|| side_from_title(&title)),
        });
    }

    if tracks.is_empty() {
        return Err(format!("{}: keine abspielbaren Tracks", cue_path.display()));
    }

    let referenced: Vec<PathBuf> = files.iter().flatten().cloned().collect();
    let cover_bytes = referenced
        .iter()
        .find_map(|path| metadata::read(path).ok().and_then(|tags| tags.picture));
    let folder_cover = artwork::find_folder_cover(&folder);
    let cover_source = artwork::bytes(cover_bytes.as_deref(), folder_cover.as_deref());
    let added_at = referenced
        .iter()
        .map(|path| mtime_secs(path))
        .chain(std::iter::once(mtime_secs(cue_path)))
        .max()
        .unwrap_or_else(super::db::now_secs);

    let folder_name = folder
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "Unbekanntes Album".to_string());
    let year = sheet
        .date
        .as_deref()
        .and_then(|date| date.get(..4))
        .and_then(|year| year.parse().ok());

    let album = assemble(
        AlbumMeta {
            title: sheet.title.clone().unwrap_or(folder_name),
            album_artist,
            year,
            genre: sheet.genre.clone(),
            label: None,
            folder,
            added_at,
        },
        tracks,
        cover_source,
        covers,
    );

    Ok(CueAlbum {
        album,
        files: referenced,
        warnings: sheet.warnings,
    })
}

fn assemble(
    meta: AlbumMeta,
    mut tracks: Vec<BuiltTrack>,
    cover_source: Option<Vec<u8>>,
    covers: &Path,
) -> Album {
    tracks.retain(|track| track.duration > 0.01);
    tracks.sort_by(|a, b| a.number.cmp(&b.number).then_with(|| a.path.cmp(&b.path)));

    if tracks.is_empty() {
        tracks.push(BuiltTrack {
            number: 1,
            title: "Unbekannter Track".to_string(),
            artist: meta.album_artist.clone(),
            duration: 0.0,
            pregap: 0.0,
            path: meta.folder.clone(),
            offset: 0.0,
            side_hint: None,
        });
    }

    let total: f64 = tracks.iter().map(BuiltTrack::span).sum();
    let format = if tracks.len() <= 2 && total <= 600.0 {
        DiscFormat::Single7
    } else {
        DiscFormat::Lp12
    };
    let rpm = match format {
        DiscFormat::Single7 => Rpm::R45,
        _ => Rpm::R33,
    };

    let album_id = short_id(&format!(
        "{}|{}|{}",
        meta.folder.display(),
        meta.title,
        meta.album_artist
    ));

    let groups = group_sides(&tracks);

    let sides: Vec<Side> = groups
        .into_iter()
        .map(|(label, group)| {
            let side_label = label.to_string();
            Side {
                label: side_label.clone(),
                rpm,
                matrix: Some(format!("LP-{}-{}", album_id[..6].to_uppercase(), label)),
                tracks: group
                    .into_iter()
                    .map(|track| Track {
                        id: short_id(&format!(
                            "{}|{}|{}|{}|{}",
                            album_id,
                            label,
                            track.number,
                            track.path.display(),
                            track.offset
                        )),
                        number: track.number,
                        title: track.title,
                        artist: track.artist,
                        duration: track.duration,
                        pregap: track.pregap,
                        path: track.path.to_string_lossy().into_owned(),
                        offset: track.offset,
                        side: side_label.clone(),
                    })
                    .collect(),
            }
        })
        .collect();

    let cover = artwork::cache(cover_source.as_deref(), None, &album_id, covers);
    let palette = cover_source
        .as_deref()
        .and_then(palette::from_bytes)
        .unwrap_or_else(Palette::fallback);

    let track_count = sides.iter().map(|side| side.tracks.len() as u32).sum();
    let duration: f64 = sides
        .iter()
        .flat_map(|side| &side.tracks)
        .map(|track| track.duration + track.pregap)
        .sum();

    Album {
        id: album_id,
        title: meta.title,
        album_artist: meta.album_artist,
        year: meta.year,
        genre: meta.genre,
        label: meta.label,
        added_at: meta.added_at,
        cover,
        palette,
        format,
        sides,
        folder: meta.folder.to_string_lossy().into_owned(),
        track_count,
        duration,
    }
}

/// Groups tracks into sides. Explicit hints (CUE side, disc number, `A1` title
/// prefix) win; otherwise split at the track boundary closest to half of the
/// record.
fn group_sides(tracks: &[BuiltTrack]) -> Vec<(char, Vec<BuiltTrack>)> {
    let distinct: BTreeSet<char> = tracks.iter().filter_map(|track| track.side_hint).collect();

    if distinct.len() >= 2 {
        let mut groups: BTreeMap<char, Vec<BuiltTrack>> = BTreeMap::new();
        let mut last = *distinct.iter().next().unwrap_or(&'A');
        for track in tracks {
            if let Some(hint) = track.side_hint {
                last = hint;
            }
            groups.entry(last).or_default().push(track.clone());
        }
        return groups.into_iter().collect();
    }

    if tracks.len() <= 1 {
        return vec![('A', tracks.to_vec())];
    }

    let spans: Vec<f64> = tracks.iter().map(BuiltTrack::span).collect();
    let split = split_index(&spans);
    let mut groups = vec![('A', tracks[..split].to_vec())];
    groups.push(('B', tracks[split..].to_vec()));
    groups
}

/// Index of the first track after the split, chosen so both halves are as
/// balanced as possible. Always returns `1..len`.
fn split_index(spans: &[f64]) -> usize {
    if spans.len() <= 1 {
        return spans.len();
    }
    let total: f64 = spans.iter().sum();
    let mut running = 0.0;
    let mut best = (f64::MAX, 1);
    for (index, span) in spans.iter().enumerate().take(spans.len() - 1) {
        running += span;
        let diff = (running - total / 2.0).abs();
        if diff < best.0 {
            best = (diff, index + 1);
        }
    }
    best.1
}

fn split_folder_name(name: &str) -> Option<(String, String)> {
    for separator in [" - ", " – ", " — "] {
        if let Some((artist, album)) = name.split_once(separator) {
            let artist = artist.trim();
            let album = album.trim();
            if !artist.is_empty() && !album.is_empty() {
                return Some((artist.to_string(), album.to_string()));
            }
        }
    }
    None
}

fn side_from_title(title: &str) -> Option<char> {
    let first = title.as_bytes().first()?.to_ascii_uppercase();
    if !(b'A'..=b'D').contains(&first) {
        return None;
    }
    let rest = title[1..].trim_start();
    rest.chars()
        .next()
        .filter(|c| c.is_ascii_digit())
        .map(|_| first as char)
}

fn disc_side(disc: u32) -> Option<char> {
    match disc {
        1 => Some('A'),
        2 => Some('B'),
        3 => Some('C'),
        4 => Some('D'),
        _ => None,
    }
}

fn short_id(input: &str) -> String {
    let hash = blake3::hash(input.as_bytes());
    hash.to_hex()[..16].to_string()
}

fn mtime_secs(path: &Path) -> i64 {
    fs::metadata(path)
        .and_then(|meta| meta.modified())
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_four_equal_tracks_in_the_middle() {
        assert_eq!(split_index(&[10.0, 10.0, 10.0, 10.0]), 2);
    }

    #[test]
    fn split_respects_uneven_tracks() {
        // 8 + 16 + 8: the closest cut is after the first track.
        assert_eq!(split_index(&[8.0, 16.0, 8.0]), 1);
    }

    #[test]
    fn split_always_keeps_both_sides_non_empty() {
        assert_eq!(split_index(&[1.0]), 1);
        assert_eq!(split_index(&[1.0, 1.0]), 1);
        assert_eq!(split_index(&[100.0, 0.1]), 1);
    }

    #[test]
    fn detects_side_prefixes() {
        assert_eq!(side_from_title("A1 Under the Broken Motel"), Some('A'));
        assert_eq!(side_from_title("b2 - Silver Engine"), Some('B'));
        assert_eq!(side_from_title("C3 Summer"), Some('C'));
        assert_eq!(side_from_title("A Day in the Life"), None);
        assert_eq!(side_from_title("01 First"), None);
    }

    #[test]
    fn splits_artist_and_album_from_folder_name() {
        assert_eq!(
            split_folder_name("Marlowe & the Drift - Low Tide Hymns"),
            Some(("Marlowe & the Drift".into(), "Low Tide Hymns".into()))
        );
        assert_eq!(split_folder_name("Low Tide Hymns"), None);
        assert_eq!(split_folder_name(" - "), None);
    }

    #[test]
    fn maps_disc_numbers_to_sides() {
        assert_eq!(disc_side(1), Some('A'));
        assert_eq!(disc_side(2), Some('B'));
        assert_eq!(disc_side(5), None);
    }

    #[test]
    fn groups_by_explicit_hints_before_time() {
        let track = |number: u32, side_hint: Option<char>| BuiltTrack {
            number,
            title: format!("Track {number}"),
            artist: "A".into(),
            duration: 10.0,
            pregap: 0.0,
            path: PathBuf::from(format!("/m/{number}.flac")),
            offset: 0.0,
            side_hint,
        };
        let groups = group_sides(&[
            track(1, Some('A')),
            track(2, Some('A')),
            track(3, Some('B')),
            track(4, Some('B')),
        ]);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].0, 'A');
        assert_eq!(groups[0].1.len(), 2);
        assert_eq!(groups[1].1.len(), 2);
    }

    #[test]
    fn falls_back_to_time_split_without_hints() {
        let make = |number: u32| BuiltTrack {
            number,
            title: format!("T{number}"),
            artist: "A".into(),
            duration: 120.0,
            pregap: 0.0,
            path: PathBuf::from(format!("/m/{number}.flac")),
            offset: 0.0,
            side_hint: None,
        };
        let groups = group_sides(&[make(1), make(2), make(3), make(4)]);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].1.len(), 2);
    }
}

#[cfg(test)]
mod scan_tests {
    use super::*;
    use std::fs;

    fn write_wav(path: &Path, seconds: f64) {
        let sample_rate: u32 = 8_000;
        let frames = (seconds * f64::from(sample_rate)) as u32;
        let data_len = frames * 2;
        let mut out = Vec::new();
        out.extend_from_slice(b"RIFF");
        out.extend_from_slice(&(36 + data_len).to_le_bytes());
        out.extend_from_slice(b"WAVEfmt ");
        out.extend_from_slice(&16u32.to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&sample_rate.to_le_bytes());
        out.extend_from_slice(&(sample_rate * 2).to_le_bytes());
        out.extend_from_slice(&2u16.to_le_bytes());
        out.extend_from_slice(&16u16.to_le_bytes());
        out.extend_from_slice(b"data");
        out.extend_from_slice(&data_len.to_le_bytes());
        out.resize(out.len() + data_len as usize, 0);
        fs::write(path, out).unwrap();
    }

    fn temp_root(name: &str) -> PathBuf {
        let stamp = std::time::SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("longplay-{name}-{}-{stamp}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn scans_files_and_cue_sheets_into_sides() {
        let root = temp_root("scan");
        let covers = root.join(".covers");

        let lp = root.join("Marlowe & the Drift - Low Tide Hymns");
        fs::create_dir_all(&lp).unwrap();
        for (name, seconds) in [
            ("A1 Under the Broken Motel.wav", 4.0),
            ("A2 The Electric Signal.wav", 6.0),
            ("B1 Summer Avenue.wav", 8.0),
            ("B2 Broken Highway.wav", 10.0),
        ] {
            write_wav(&lp.join(name), seconds);
        }

        let single = root.join("June Avery - Paper Moons");
        fs::create_dir_all(&single).unwrap();
        write_wav(&single.join("01 Paper Moons.wav"), 5.0);
        write_wav(&single.join("02 Afterglow.wav"), 5.0);

        let cue_dir = root.join("Cue Album");
        fs::create_dir_all(&cue_dir).unwrap();
        write_wav(&cue_dir.join("album.wav"), 10.0);
        fs::write(
            cue_dir.join("album.cue"),
            "PERFORMER \"Cue Artist\"\nTITLE \"Cue Album\"\nFILE \"album.wav\" WAVE\n  \
             TRACK 01 AUDIO\n    TITLE \"One\"\n    INDEX 00 00:00:00\n    INDEX 01 00:00:00\n  \
             TRACK 02 AUDIO\n    TITLE \"Two\"\n    INDEX 00 00:04:00\n    INDEX 01 00:05:00\n",
        )
        .unwrap();

        let mut db = Db::in_memory().unwrap();
        let summary = scan(std::slice::from_ref(&root), &mut db, &covers).unwrap();
        assert_eq!(summary.albums, 3);
        assert!(summary.issues.is_empty(), "{:?}", summary.issues);
        assert_eq!(summary.tracks, 8);

        let albums = db.load_albums().unwrap();
        let lp_album = albums
            .iter()
            .find(|album| album.title.contains("Low Tide"))
            .expect("lp album");
        assert_eq!(lp_album.sides.len(), 2);
        assert_eq!(lp_album.sides[0].tracks.len(), 2);
        assert_eq!(lp_album.sides[1].tracks.len(), 2);
        assert_eq!(lp_album.sides[0].label, "A");
        assert_eq!(lp_album.sides[1].label, "B");
        assert_eq!(lp_album.format, DiscFormat::Lp12);
        assert!((lp_album.duration - 28.0).abs() < 0.2);

        let single_album = albums
            .iter()
            .find(|album| album.title.contains("Paper Moons"))
            .expect("single");
        assert_eq!(single_album.format, DiscFormat::Single7);
        assert_eq!(single_album.sides[0].rpm, Rpm::R45);
        assert_eq!(single_album.sides.len(), 2);

        let cue_album = albums
            .iter()
            .find(|album| album.title == "Cue Album")
            .expect("cue album");
        assert_eq!(cue_album.album_artist, "Cue Artist");
        assert_eq!(cue_album.track_count, 2);
        let cue_tracks: Vec<_> = cue_album
            .sides
            .iter()
            .flat_map(|side| &side.tracks)
            .collect();
        assert_eq!(cue_tracks[0].title, "One");
        assert!((cue_tracks[1].pregap - 1.0).abs() < 1e-6);
        assert!((cue_tracks[1].offset - 5.0).abs() < 1e-6);

        let ids: Vec<_> = albums.iter().map(|album| album.id.clone()).collect();
        scan(std::slice::from_ref(&root), &mut db, &covers).unwrap();
        let again: Vec<_> = db
            .load_albums()
            .unwrap()
            .iter()
            .map(|album| album.id.clone())
            .collect();
        assert_eq!(ids, again, "album ids must be stable across scans");

        fs::remove_dir_all(&root).ok();
    }
}
