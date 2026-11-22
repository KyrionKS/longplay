//! CUE sheet parsing.
//!
//! Parses a CUE sheet into a [`CueSheet`] with file and track metadata. The
//! parser is deliberately forgiving: only an empty document and documents
//! without any usable `TRACK` line are errors, everything else is reported as
//! a warning on the sheet.

use std::collections::{HashMap, HashSet};
use std::fmt;

/// A parsed CUE sheet.
#[derive(Debug, Clone, PartialEq)]
pub struct CueSheet {
    pub title: Option<String>,
    pub performer: Option<String>,
    pub genre: Option<String>,
    pub date: Option<String>,
    pub files: Vec<CueFile>,
    pub tracks: Vec<CueTrack>,
    pub warnings: Vec<String>,
}

/// A `FILE` entry from a CUE sheet.
#[derive(Debug, Clone, PartialEq)]
pub struct CueFile {
    pub name: String,
    pub kind: Option<String>,
}

/// A `TRACK` entry from a CUE sheet.
#[derive(Debug, Clone, PartialEq)]
pub struct CueTrack {
    pub number: u32,
    pub kind: String,
    pub title: Option<String>,
    pub performer: Option<String>,
    pub file_index: Option<usize>,
    pub index00: Option<f64>,
    pub index01: Option<f64>,
    pub side: Option<char>,
}

impl CueTrack {
    /// Length of the pregap in seconds.
    ///
    /// Returns `0.0` when either index is missing or the pregap is negative.
    pub fn pregap(&self) -> f64 {
        match (self.index00, self.index01) {
            (Some(start), Some(end)) if end > start => end - start,
            _ => 0.0,
        }
    }

    /// Track start in seconds, taken from `INDEX 01`.
    pub fn start(&self) -> Option<f64> {
        self.index01
    }
}

/// Errors that prevent a CUE sheet from being parsed at all.
#[derive(Debug, Clone, PartialEq)]
pub enum CueError {
    Empty,
    NoTracks,
    InvalidEncoding,
}

impl fmt::Display for CueError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            CueError::Empty => "CUE sheet is empty",
            CueError::NoTracks => "CUE sheet contains no tracks",
            CueError::InvalidEncoding => "CUE sheet has an invalid encoding",
        };
        f.write_str(message)
    }
}

impl std::error::Error for CueError {}

/// Parses a CUE sheet from raw bytes.
///
/// A UTF-8 BOM is stripped. Valid UTF-8 is used as-is, otherwise the input is
/// decoded as Windows-1252.
pub fn parse_bytes(bytes: &[u8]) -> Result<CueSheet, CueError> {
    let bytes = if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        &bytes[3..]
    } else {
        bytes
    };

    if bytes.is_empty() {
        return Err(CueError::Empty);
    }

    match std::str::from_utf8(bytes) {
        Ok(text) => parse_str(text),
        Err(_) => parse_str(&decode_cp1252(bytes)),
    }
}

/// Parses a CUE sheet from text.
pub fn parse_str(text: &str) -> Result<CueSheet, CueError> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);

    if text.is_empty() {
        return Err(CueError::Empty);
    }

    let mut sheet = CueSheet {
        title: None,
        performer: None,
        genre: None,
        date: None,
        files: Vec::new(),
        tracks: Vec::new(),
        warnings: Vec::new(),
    };

    // Index of the FILE currently in effect.
    let mut current_file: Option<usize> = None;
    // Index of the TRACK currently receiving TITLE/PERFORMER/INDEX lines.
    let mut current_track: Option<usize> = None;
    // Side set through `REM SIDE`, applies to subsequent tracks.
    let mut explicit_side: Option<char> = None;
    // Side derived from `REM DISCNUMBER`.
    let mut disc_side: Option<char> = None;
    // Last INDEX 01 per file, used to detect backwards running time.
    let mut last_index01: HashMap<Option<usize>, f64> = HashMap::new();
    let mut seen_numbers: HashSet<u32> = HashSet::new();

    for raw_line in text.lines() {
        let line = raw_line.trim();

        if line.is_empty() || line.starts_with(';') {
            continue;
        }

        let (keyword, rest) = split_keyword(line);

        match keyword.to_ascii_uppercase().as_str() {
            "FILE" => {
                let (name, kind) = parse_file_args(rest);
                sheet.files.push(CueFile { name, kind });
                current_file = Some(sheet.files.len() - 1);
                current_track = None;
            }
            "TRACK" => {
                current_track = None;

                let (number_token, kind) = split_keyword(rest);
                let Ok(number) = number_token.parse::<u32>() else {
                    sheet
                        .warnings
                        .push(format!("invalid TRACK line: \"{line}\""));
                    continue;
                };

                if !seen_numbers.insert(number) {
                    sheet
                        .warnings
                        .push(format!("duplicate track number {number}"));
                }

                if current_file.is_none() {
                    sheet
                        .warnings
                        .push(format!("track {number}: no FILE before track"));
                }

                sheet.tracks.push(CueTrack {
                    number,
                    kind: kind.to_string(),
                    title: None,
                    performer: None,
                    file_index: current_file,
                    index00: None,
                    index01: None,
                    side: explicit_side.or(disc_side),
                });
                current_track = Some(sheet.tracks.len() - 1);
            }
            "TITLE" => {
                let value = parse_value(rest);

                if let Some(index) = current_track {
                    if sheet.tracks[index].title.is_none() {
                        let prefix_side = side_from_title(&value);
                        let track = &mut sheet.tracks[index];
                        track.title = Some(value);

                        if explicit_side.is_none()
                            && let Some(side) = prefix_side
                        {
                            track.side = Some(side);
                        }
                    }
                } else if sheet.title.is_none() {
                    sheet.title = Some(value);
                }
            }
            "PERFORMER" => {
                let value = parse_value(rest);

                if let Some(index) = current_track {
                    if sheet.tracks[index].performer.is_none() {
                        sheet.tracks[index].performer = Some(value);
                    }
                } else if sheet.performer.is_none() {
                    sheet.performer = Some(value);
                }
            }
            "REM" => {
                let (key, value_rest) = split_keyword(rest);
                let value = parse_value(value_rest);

                match key.to_ascii_uppercase().as_str() {
                    "SIDE" => {
                        if let Some(side) = side_char(&value) {
                            explicit_side = Some(side);
                            // A later REM SIDE also applies to tracks that have
                            // no side yet.
                            for track in &mut sheet.tracks {
                                if track.side.is_none() {
                                    track.side = Some(side);
                                }
                            }
                        } else {
                            sheet
                                .warnings
                                .push(format!("invalid REM SIDE value \"{value}\""));
                        }
                    }
                    "GENRE" => {
                        if sheet.genre.is_none() && !value.is_empty() {
                            sheet.genre = Some(value);
                        }
                    }
                    "DATE" => {
                        if sheet.date.is_none() && !value.is_empty() {
                            sheet.date = Some(value);
                        }
                    }
                    "DISCNUMBER" => {
                        if let Some(side) = side_from_disc(&value) {
                            disc_side = Some(side);
                        }
                    }
                    _ => {}
                }
            }
            "INDEX" => {
                let (number_token, time_rest) = split_keyword(rest);
                let Ok(index_number) = number_token.parse::<u32>() else {
                    continue;
                };

                if index_number > 1 {
                    continue;
                }

                let Some(track_index) = current_track else {
                    continue;
                };

                let time_token = time_rest.split_whitespace().next().unwrap_or("");
                let parsed = parse_time(time_token);
                let number = sheet.tracks[track_index].number;

                match (index_number, parsed) {
                    (0, Some(value)) => {
                        sheet.tracks[track_index].index00 = Some(value);
                    }
                    (1, Some(value)) => {
                        let file_index = sheet.tracks[track_index].file_index;
                        if let Some(previous) = last_index01.get(&file_index)
                            && value < *previous
                        {
                            sheet
                                .warnings
                                .push(format!("track {number}: INDEX 01 moves backwards"));
                        }
                        last_index01.insert(file_index, value);
                        sheet.tracks[track_index].index01 = Some(value);
                    }
                    (_, None) => {
                        sheet.warnings.push(format!(
                            "track {number}: invalid INDEX {index_number:02} time \"{time_token}\""
                        ));
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    if sheet.tracks.is_empty() {
        return Err(CueError::NoTracks);
    }

    for track in &sheet.tracks {
        if track.index01.is_none() {
            sheet
                .warnings
                .push(format!("track {}: missing INDEX 01", track.number));
        }
    }

    Ok(sheet)
}

/// Splits the first whitespace-delimited token from the rest of a line.
fn split_keyword(line: &str) -> (&str, &str) {
    let line = line.trim();

    match line.find(char::is_whitespace) {
        Some(position) => (&line[..position], line[position..].trim()),
        None => (line, ""),
    }
}

/// Parses a value that may be wrapped in double quotes.
fn parse_value(rest: &str) -> String {
    let rest = rest.trim();

    if let Some(after_quote) = rest.strip_prefix('"') {
        match after_quote.find('"') {
            Some(end) => after_quote[..end].to_string(),
            None => after_quote.to_string(),
        }
    } else {
        rest.to_string()
    }
}

/// Parses the arguments of a `FILE` line into name and optional kind.
fn parse_file_args(rest: &str) -> (String, Option<String>) {
    let rest = rest.trim();

    if let Some(after_quote) = rest.strip_prefix('"') {
        if let Some(end) = after_quote.find('"') {
            let name = after_quote[..end].to_string();
            let kind = after_quote[end + 1..]
                .split_whitespace()
                .next()
                .map(str::to_string);
            return (name, kind);
        }

        return (after_quote.to_string(), None);
    }

    let mut parts = rest.split_whitespace();
    let name = parts.next().unwrap_or("").to_string();
    let kind = parts.next().map(str::to_string);
    (name, kind)
}

/// Parses `mm:ss:ff` into seconds using 75 frames per second.
fn parse_time(value: &str) -> Option<f64> {
    let value = value.trim().trim_matches('"');
    let mut parts = value.split(':');

    let minutes = parts.next()?.parse::<u32>().ok()?;
    let seconds = parts.next()?.parse::<u32>().ok()?;
    let frames = parts.next()?.parse::<u32>().ok()?;

    if parts.next().is_some() || seconds > 59 || frames > 74 {
        return None;
    }

    Some(minutes as f64 * 60.0 + seconds as f64 + frames as f64 / 75.0)
}

/// Extracts a side letter from a `REM SIDE` value.
fn side_char(value: &str) -> Option<char> {
    let side = value.trim().chars().next()?.to_ascii_uppercase();

    if matches!(side, 'A'..='D') {
        Some(side)
    } else {
        None
    }
}

/// Maps a `REM DISCNUMBER` value like `"1/2"` to a side letter.
fn side_from_disc(value: &str) -> Option<char> {
    let disc = value.split('/').next()?.trim().parse::<u32>().ok()?;

    match disc {
        1 => Some('A'),
        2 => Some('B'),
        3 => Some('C'),
        4 => Some('D'),
        _ => None,
    }
}

/// Detects a side prefix in a track title such as `A1`, `B2` or `A 1`.
fn side_from_title(title: &str) -> Option<char> {
    let first = title.chars().next()?;
    let side = first.to_ascii_uppercase();

    if !matches!(side, 'A'..='D') {
        return None;
    }

    let rest = title[first.len_utf8()..].trim_start();

    if rest.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        Some(side)
    } else {
        None
    }
}

/// Decodes bytes as Windows-1252.
fn decode_cp1252(bytes: &[u8]) -> String {
    bytes.iter().map(|&byte| cp1252_char(byte)).collect()
}

/// Maps a single byte to its Windows-1252 character.
fn cp1252_char(byte: u8) -> char {
    // 0x80..=0x9F differ from Latin-1. Undefined positions keep their control
    // character code point.
    const HIGH: [char; 32] = [
        '\u{20AC}', '\u{0081}', '\u{201A}', '\u{0192}', '\u{201E}', '\u{2026}', '\u{2020}',
        '\u{2021}', '\u{02C6}', '\u{2030}', '\u{0160}', '\u{2039}', '\u{0152}', '\u{008D}',
        '\u{017D}', '\u{008F}', '\u{0090}', '\u{2018}', '\u{2019}', '\u{201C}', '\u{201D}',
        '\u{2022}', '\u{2013}', '\u{2014}', '\u{02DC}', '\u{2122}', '\u{0161}', '\u{203A}',
        '\u{0153}', '\u{009D}', '\u{017E}', '\u{0178}',
    ];

    match byte {
        0x80..=0x9F => HIGH[(byte - 0x80) as usize],
        _ => byte as char,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIMPLE: &str = "\
TITLE \"Album\"
PERFORMER \"Artist\"
FILE \"album.wav\" WAVE
  TRACK 01 AUDIO
    TITLE \"First Song\"
    PERFORMER \"Someone\"
    INDEX 01 00:00:00
";

    #[test]
    fn parses_single_file_sheet() {
        let sheet = parse_str(SIMPLE).unwrap();

        assert_eq!(sheet.title.as_deref(), Some("Album"));
        assert_eq!(sheet.performer.as_deref(), Some("Artist"));
        assert_eq!(
            sheet.files,
            vec![CueFile {
                name: "album.wav".to_string(),
                kind: Some("WAVE".to_string()),
            }]
        );
        assert_eq!(sheet.tracks.len(), 1);

        let track = &sheet.tracks[0];
        assert_eq!(track.number, 1);
        assert_eq!(track.kind, "AUDIO");
        assert_eq!(track.title.as_deref(), Some("First Song"));
        assert_eq!(track.performer.as_deref(), Some("Someone"));
        assert_eq!(track.file_index, Some(0));
        assert_eq!(track.start(), Some(0.0));
        assert!(sheet.warnings.is_empty());
    }

    #[test]
    fn parses_multiple_files_and_links_tracks() {
        let sheet = parse_str(
            "\
FILE \"one.wav\" WAVE
  TRACK 01 AUDIO
    INDEX 01 00:00:00
FILE \"two.mp3\" MP3
  TRACK 02 AUDIO
    INDEX 01 00:00:00
",
        )
        .unwrap();

        assert_eq!(sheet.files.len(), 2);
        assert_eq!(sheet.files[1].name, "two.mp3");
        assert_eq!(sheet.files[1].kind.as_deref(), Some("MP3"));
        assert_eq!(sheet.tracks[0].file_index, Some(0));
        assert_eq!(sheet.tracks[1].file_index, Some(1));
    }

    #[test]
    fn computes_pregap_and_rejects_negative_pregap() {
        let sheet = parse_str(
            "\
FILE \"a.wav\" WAVE
  TRACK 01 AUDIO
    INDEX 00 00:01:00
    INDEX 01 00:03:00
  TRACK 02 AUDIO
    INDEX 00 00:04:00
    INDEX 01 00:03:00
  TRACK 03 AUDIO
    INDEX 01 00:05:00
",
        )
        .unwrap();

        assert_eq!(sheet.tracks[0].pregap(), 2.0);
        assert_eq!(sheet.tracks[0].start(), Some(3.0));
        assert_eq!(sheet.tracks[1].pregap(), 0.0);
        assert_eq!(sheet.tracks[2].pregap(), 0.0);
    }

    #[test]
    fn parses_quoted_values_with_spaces() {
        let sheet = parse_str(
            "\
TITLE \"My Great Album\"
PERFORMER \"The Band\"
FILE \"my file.wav\" BINARY
  TRACK 01 AUDIO
    TITLE \"Song With Spaces\"
    INDEX 01 00:00:00
",
        )
        .unwrap();

        assert_eq!(sheet.title.as_deref(), Some("My Great Album"));
        assert_eq!(sheet.performer.as_deref(), Some("The Band"));
        assert_eq!(sheet.files[0].name, "my file.wav");
        assert_eq!(sheet.files[0].kind.as_deref(), Some("BINARY"));
        assert_eq!(sheet.tracks[0].title.as_deref(), Some("Song With Spaces"));
    }

    #[test]
    fn strips_utf8_bom() {
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(SIMPLE.as_bytes());

        let sheet = parse_bytes(&bytes).unwrap();
        assert_eq!(sheet.title.as_deref(), Some("Album"));
    }

    #[test]
    fn decodes_windows_1252_fallback() {
        let mut bytes = b"FILE \"x.wav\" WAVE\nTRACK 01 AUDIO\nTITLE \"Caf".to_vec();
        bytes.push(0xE9); // é in CP1252 / Latin-1
        bytes.extend_from_slice(b" \x80\"\nINDEX 01 00:00:00\n");
        // 0x80 is the euro sign in CP1252.

        let sheet = parse_bytes(&bytes).unwrap();
        assert_eq!(sheet.tracks[0].title.as_deref(), Some("Café €"));
    }

    #[test]
    fn warns_about_missing_index_01() {
        let sheet = parse_str(
            "\
FILE \"a.wav\" WAVE
  TRACK 01 AUDIO
    INDEX 00 00:00:00
",
        )
        .unwrap();

        assert!(
            sheet
                .warnings
                .contains(&"track 1: missing INDEX 01".to_string())
        );
        assert_eq!(sheet.tracks[0].index01, None);
        assert_eq!(sheet.tracks[0].start(), None);
    }

    #[test]
    fn warns_about_invalid_index_time() {
        let sheet = parse_str(
            "\
FILE \"a.wav\" WAVE
  TRACK 01 AUDIO
    INDEX 01 00:60:00
  TRACK 02 AUDIO
    INDEX 01 00:00:75
",
        )
        .unwrap();

        assert!(
            sheet
                .warnings
                .iter()
                .any(|warning| warning.contains("invalid INDEX 01"))
        );
        assert_eq!(sheet.tracks[0].index01, None);
        assert_eq!(sheet.tracks[1].index01, None);
    }

    #[test]
    fn warns_about_duplicate_track_numbers() {
        let sheet = parse_str(
            "\
FILE \"a.wav\" WAVE
  TRACK 01 AUDIO
    INDEX 01 00:00:00
  TRACK 01 AUDIO
    INDEX 01 00:01:00
",
        )
        .unwrap();

        assert!(
            sheet
                .warnings
                .contains(&"duplicate track number 1".to_string())
        );
        assert_eq!(sheet.tracks.len(), 2);
    }

    #[test]
    fn rem_side_applies_to_following_tracks() {
        let sheet = parse_str(
            "\
REM SIDE \"B\"
FILE \"a.wav\" WAVE
  TRACK 01 AUDIO
    INDEX 01 00:00:00
  TRACK 02 AUDIO
    INDEX 01 00:01:00
",
        )
        .unwrap();

        assert_eq!(sheet.tracks[0].side, Some('B'));
        assert_eq!(sheet.tracks[1].side, Some('B'));
    }

    #[test]
    fn rem_side_applies_to_already_parsed_tracks_without_side() {
        let sheet = parse_str(
            "\
FILE \"a.wav\" WAVE
  TRACK 01 AUDIO
    TITLE \"Intro\"
    INDEX 01 00:00:00
REM SIDE \"D\"
",
        )
        .unwrap();

        assert_eq!(sheet.tracks[0].side, Some('D'));
    }

    #[test]
    fn discnumber_selects_side() {
        let sheet = parse_str(
            "\
REM DISCNUMBER \"2/2\"
FILE \"a.wav\" WAVE
  TRACK 01 AUDIO
    INDEX 01 00:00:00
",
        )
        .unwrap();

        assert_eq!(sheet.tracks[0].side, Some('B'));

        let third = parse_str(
            "\
REM DISCNUMBER 3/3
FILE \"a.wav\" WAVE
  TRACK 01 AUDIO
    INDEX 01 00:00:00
",
        )
        .unwrap();
        assert_eq!(third.tracks[0].side, Some('C'));
    }

    #[test]
    fn title_prefix_detects_side_without_removing_prefix() {
        let sheet = parse_str(
            "\
FILE \"a.wav\" WAVE
  TRACK 01 AUDIO
    TITLE \"A1 Intro\"
    INDEX 01 00:00:00
  TRACK 02 AUDIO
    TITLE \"B 2 Outro\"
    INDEX 01 00:01:00
  TRACK 03 AUDIO
    TITLE \"Coda\"
    INDEX 01 00:02:00
",
        )
        .unwrap();

        assert_eq!(sheet.tracks[0].side, Some('A'));
        assert_eq!(sheet.tracks[1].side, Some('B'));
        assert_eq!(sheet.tracks[2].side, None);
        // The prefix is deliberately kept in the title.
        assert_eq!(sheet.tracks[0].title.as_deref(), Some("A1 Intro"));
    }

    #[test]
    fn explicit_side_wins_over_title_prefix() {
        let sheet = parse_str(
            "\
REM SIDE \"C\"
FILE \"a.wav\" WAVE
  TRACK 01 AUDIO
    TITLE \"A1 Intro\"
    INDEX 01 00:00:00
",
        )
        .unwrap();

        assert_eq!(sheet.tracks[0].side, Some('C'));
    }

    #[test]
    fn empty_input_is_an_error() {
        assert_eq!(parse_str(""), Err(CueError::Empty));
        assert_eq!(parse_bytes(&[]), Err(CueError::Empty));
    }

    #[test]
    fn sheet_without_tracks_is_an_error() {
        assert_eq!(
            parse_str("TITLE \"Album\"\nPERFORMER \"Artist\"\n"),
            Err(CueError::NoTracks)
        );
    }

    #[test]
    fn keywords_are_case_insensitive() {
        let sheet = parse_str(
            "\
title \"Album\"
performer \"Artist\"
file \"a.wav\" wav
track 01 audio
title \"Song\"
index 01 00:00:10
rem genre \"Rock\"
",
        )
        .unwrap();

        assert_eq!(sheet.title.as_deref(), Some("Album"));
        assert_eq!(sheet.performer.as_deref(), Some("Artist"));
        assert_eq!(sheet.tracks[0].kind, "audio");
        assert_eq!(sheet.tracks[0].title.as_deref(), Some("Song"));
        assert_eq!(sheet.genre.as_deref(), Some("Rock"));

        let start = sheet.tracks[0].start().unwrap();
        assert!((start - 10.0 / 75.0).abs() < 1e-9);
    }

    #[test]
    fn blank_lines_comments_and_unknown_keywords_are_ignored() {
        let sheet = parse_str(
            "\
; a comment
   ; an indented comment

CATALOG 123456789
FILE \"a.wav\" WAVE
  TRACK 01 AUDIO
    INDEX 01 00:00:00
",
        )
        .unwrap();

        assert_eq!(sheet.tracks.len(), 1);
        assert!(sheet.warnings.is_empty());
    }

    #[test]
    fn track_without_file_warns() {
        let sheet = parse_str(
            "\
TRACK 01 AUDIO
  INDEX 01 00:00:00
",
        )
        .unwrap();

        assert!(
            sheet
                .warnings
                .contains(&"track 1: no FILE before track".to_string())
        );
        assert_eq!(sheet.tracks[0].file_index, None);
        assert!(sheet.files.is_empty());
    }

    #[test]
    fn warns_when_time_runs_backwards_within_a_file() {
        let sheet = parse_str(
            "\
FILE \"a.wav\" WAVE
  TRACK 01 AUDIO
    INDEX 01 00:10:00
  TRACK 02 AUDIO
    INDEX 01 00:05:00
",
        )
        .unwrap();

        assert!(
            sheet
                .warnings
                .iter()
                .any(|warning| warning.contains("moves backwards"))
        );
    }

    #[test]
    fn rem_genre_and_date_fill_empty_sheet_fields() {
        let sheet = parse_str(
            "\
REM GENRE \"Rock\"
REM DATE \"1999\"
REM GENRE \"Jazz\"
FILE \"a.wav\" WAVE
  TRACK 01 AUDIO
    INDEX 01 00:00:00
",
        )
        .unwrap();

        assert_eq!(sheet.genre.as_deref(), Some("Rock"));
        assert_eq!(sheet.date.as_deref(), Some("1999"));
    }
}
