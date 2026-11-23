//! M3U/M3U8 playlist parsing.
//!
//! The parser is deliberately forgiving: it never fails. Invalid UTF-8 is
//! decoded lossily, comment lines are skipped and malformed `#EXTINF` lines
//! simply leave the title or duration unset. Paths are taken verbatim from the
//! playlist and are not resolved.

use std::path::PathBuf;

/// A parsed M3U/M3U8 playlist.
#[derive(Debug, Clone, PartialEq)]
pub struct M3uPlaylist {
    pub entries: Vec<M3uEntry>,
}

/// A single playlist entry.
#[derive(Debug, Clone, PartialEq)]
pub struct M3uEntry {
    /// Path as written in the playlist, not resolved.
    pub path: PathBuf,
    /// Title from the preceding `#EXTINF` line.
    pub title: Option<String>,
    /// Duration in seconds from the preceding `#EXTINF` line.
    pub duration: Option<f64>,
}

/// Parses an M3U or M3U8 playlist.
pub fn parse(bytes: &[u8]) -> M3uPlaylist {
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
    let text = String::from_utf8_lossy(bytes);

    let mut entries = Vec::new();
    let mut pending_title: Option<String> = None;
    let mut pending_duration: Option<f64> = None;

    for raw_line in text.split('\n') {
        let line = raw_line.trim_end_matches('\r').trim();
        if line.is_empty() {
            continue;
        }
        if let Some(info) = line.strip_prefix("#EXTINF:") {
            let (duration, title) = match info.split_once(',') {
                Some((duration, title)) => (duration, Some(title)),
                None => (info, None),
            };
            pending_duration = duration.trim().parse::<f64>().ok();
            pending_title = title
                .map(str::trim)
                .filter(|title| !title.is_empty())
                .map(str::to_owned);
            continue;
        }
        if line.starts_with('#') {
            continue;
        }

        entries.push(M3uEntry {
            path: PathBuf::from(line),
            title: pending_title.take(),
            duration: pending_duration.take(),
        });
    }

    M3uPlaylist { entries }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paths(playlist: &M3uPlaylist) -> Vec<String> {
        playlist
            .entries
            .iter()
            .map(|entry| entry.path.to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn parses_simple_list() {
        let playlist = parse(b"a.mp3\nb.mp3\n");
        assert_eq!(paths(&playlist), ["a.mp3", "b.mp3"]);
        assert!(playlist.entries.iter().all(|e| e.title.is_none()));
        assert!(playlist.entries.iter().all(|e| e.duration.is_none()));
    }

    #[test]
    fn parses_crlf_line_endings() {
        let playlist = parse(b"a.mp3\r\nb.mp3\r\n");
        assert_eq!(paths(&playlist), ["a.mp3", "b.mp3"]);
    }

    #[test]
    fn strips_utf8_bom() {
        let playlist = parse(b"\xEF\xBB\xBF#EXTM3U\r\nfirst.flac\r\n");
        assert_eq!(paths(&playlist), ["first.flac"]);
    }

    #[test]
    fn decodes_invalid_utf8_lossily() {
        let playlist = parse(b"\xFFbad.mp3\n");
        assert_eq!(paths(&playlist), ["\u{FFFD}bad.mp3"]);
    }

    #[test]
    fn parses_extinf_with_title_and_duration() {
        let playlist = parse(b"#EXTINF:123,Artist - Track\ntrack.mp3\n");
        assert_eq!(playlist.entries.len(), 1);
        assert_eq!(playlist.entries[0].duration, Some(123.0));
        assert_eq!(
            playlist.entries[0].title.as_deref(),
            Some("Artist - Track")
        );
    }

    #[test]
    fn parses_decimal_duration() {
        let playlist = parse(b"#EXTINF:12.75,Short\nshort.mp3\n");
        assert_eq!(playlist.entries[0].duration, Some(12.75));
    }

    #[test]
    fn parses_extinf_without_title() {
        let playlist = parse(b"#EXTINF:42,\nuntitled.mp3\n");
        assert_eq!(playlist.entries[0].duration, Some(42.0));
        assert_eq!(playlist.entries[0].title, None);
    }

    #[test]
    fn parses_extinf_without_comma() {
        let playlist = parse(b"#EXTINF:7\nno-comma.mp3\n");
        assert_eq!(playlist.entries[0].duration, Some(7.0));
        assert_eq!(playlist.entries[0].title, None);
    }

    #[test]
    fn invalid_duration_is_none() {
        let playlist = parse(b"#EXTINF:abc,Still Titled\nx.mp3\n");
        assert_eq!(playlist.entries[0].duration, None);
        assert_eq!(playlist.entries[0].title.as_deref(), Some("Still Titled"));
    }

    #[test]
    fn ignores_comments_and_blank_lines() {
        let source = b"#EXTM3U\n\n# just a comment\n   \na.mp3\n# trailing comment\n\nb.mp3\n";
        let playlist = parse(source);
        assert_eq!(paths(&playlist), ["a.mp3", "b.mp3"]);
    }

    #[test]
    fn comment_between_extinf_and_path_keeps_metadata() {
        let playlist = parse(b"#EXTINF:5,Titled\n#EXTALBUM:Album\nsong.mp3\n");
        assert_eq!(playlist.entries.len(), 1);
        assert_eq!(playlist.entries[0].title.as_deref(), Some("Titled"));
        assert_eq!(playlist.entries[0].duration, Some(5.0));
    }

    #[test]
    fn keeps_paths_with_spaces_and_trims_edges() {
        let playlist = parse(b"  /music/my song.flac  \n");
        assert_eq!(paths(&playlist), ["/music/my song.flac"]);
    }

    #[test]
    fn keeps_absolute_relative_and_windows_paths() {
        let source = b"/abs/song.flac\nrel/song.mp3\nC:\\Musik\\x.flac\n";
        let playlist = parse(source);
        assert_eq!(
            paths(&playlist),
            ["/abs/song.flac", "rel/song.mp3", "C:\\Musik\\x.flac"]
        );
    }

    #[test]
    fn resets_metadata_after_path() {
        let playlist = parse(b"#EXTINF:3,First\none.mp3\ntwo.mp3\n");
        assert_eq!(playlist.entries[0].title.as_deref(), Some("First"));
        assert_eq!(playlist.entries[1].title, None);
        assert_eq!(playlist.entries[1].duration, None);
    }

    #[test]
    fn uses_multiple_extinf_lines_in_order() {
        let source = b"#EXTM3U\n#EXTINF:1,One\none.mp3\n#EXTINF:2.5,Two\ntwo.mp3\n#EXTINF:3,Three\nthree.mp3\n";
        let playlist = parse(source);
        assert_eq!(paths(&playlist), ["one.mp3", "two.mp3", "three.mp3"]);
        assert_eq!(playlist.entries[0].duration, Some(1.0));
        assert_eq!(playlist.entries[1].duration, Some(2.5));
        assert_eq!(playlist.entries[2].duration, Some(3.0));
        assert_eq!(playlist.entries[2].title.as_deref(), Some("Three"));
    }

    #[test]
    fn empty_input_yields_empty_playlist() {
        assert_eq!(parse(b"").entries, Vec::new());
        assert_eq!(parse(b"#EXTM3U\n\n# nothing\n").entries, Vec::new());
    }
}
