//! Side timeline: converts between side time, tracks and gaps.
//!
//! A side is a sequence of `pregap + track` blocks. `start` is the first audio
//! sample of a track, after its pregap.

use crate::library::model::Side;

#[derive(Debug, Clone, PartialEq)]
pub struct TrackSpan {
    pub index: usize,
    pub number: u32,
    pub title: String,
    pub artist: String,
    pub pregap: f64,
    pub start: f64,
    pub duration: f64,
}

#[derive(Debug, Clone)]
pub struct SideTimeline {
    pub label: String,
    pub tracks: Vec<TrackSpan>,
    pub total: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrackPosition {
    pub index: usize,
    pub offset: f64,
    pub in_gap: bool,
}

impl SideTimeline {
    pub fn from_side(side: &Side) -> Self {
        let mut cursor = 0.0;
        let tracks = side
            .tracks
            .iter()
            .enumerate()
            .map(|(index, track)| {
                let pregap = track.pregap.clamp(0.0, 10.0);
                let start = cursor + pregap;
                cursor = start + track.duration.max(0.0);
                TrackSpan {
                    index,
                    number: track.number,
                    title: track.title.clone(),
                    artist: track.artist.clone(),
                    pregap,
                    start,
                    duration: track.duration.max(0.0),
                }
            })
            .collect();

        Self {
            label: side.label.clone(),
            tracks,
            total: cursor,
        }
    }

    pub fn position_of_track(&self, index: usize) -> f64 {
        self.tracks
            .get(index)
            .map(|track| track.start)
            .unwrap_or(self.total)
    }

    pub fn track_at(&self, position: f64) -> Option<TrackPosition> {
        let first = self.tracks.first()?;
        let position = position.clamp(0.0, self.total);

        for track in &self.tracks {
            if position < track.start {
                let gap_start = track.start - track.pregap;
                return Some(TrackPosition {
                    index: track.index,
                    offset: (position - gap_start).max(0.0),
                    in_gap: true,
                });
            }
            if position < track.start + track.duration {
                return Some(TrackPosition {
                    index: track.index,
                    offset: position - track.start,
                    in_gap: false,
                });
            }
        }

        Some(TrackPosition {
            index: self.tracks.last().map(|t| t.index).unwrap_or(first.index),
            offset: self.tracks.last().map(|t| t.duration).unwrap_or(0.0),
            in_gap: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::model::{Rpm, Track};

    fn side() -> Side {
        Side {
            label: "A".into(),
            rpm: Rpm::R33,
            matrix: None,
            tracks: vec![
                Track {
                    id: "a".into(),
                    number: 1,
                    title: "One".into(),
                    artist: "X".into(),
                    duration: 100.0,
                    pregap: 1.5,
                    path: "/a.flac".into(),
                    offset: 0.0,
                    side: "A".into(),
                },
                Track {
                    id: "b".into(),
                    number: 2,
                    title: "Two".into(),
                    artist: "X".into(),
                    duration: 50.0,
                    pregap: 1.5,
                    path: "/b.flac".into(),
                    offset: 0.0,
                    side: "A".into(),
                },
            ],
        }
    }

    #[test]
    fn accounts_for_pregaps() {
        let timeline = SideTimeline::from_side(&side());
        assert!((timeline.total - 153.0).abs() < 1e-9);
        assert!((timeline.tracks[0].start - 1.5).abs() < 1e-9);
        assert!((timeline.tracks[1].start - 103.0).abs() < 1e-9);
    }

    #[test]
    fn maps_positions_to_tracks_and_gaps() {
        let timeline = SideTimeline::from_side(&side());
        assert_eq!(
            timeline.track_at(0.0),
            Some(TrackPosition {
                index: 0,
                offset: 0.0,
                in_gap: true
            })
        );
        assert_eq!(
            timeline.track_at(1.5),
            Some(TrackPosition {
                index: 0,
                offset: 0.0,
                in_gap: false
            })
        );
        assert_eq!(
            timeline.track_at(50.0),
            Some(TrackPosition {
                index: 0,
                offset: 48.5,
                in_gap: false
            })
        );
        assert_eq!(
            timeline.track_at(102.0),
            Some(TrackPosition {
                index: 1,
                offset: 0.5,
                in_gap: true
            })
        );
        assert_eq!(
            timeline.track_at(103.0),
            Some(TrackPosition {
                index: 1,
                offset: 0.0,
                in_gap: false
            })
        );
        assert_eq!(
            timeline.track_at(999.0),
            Some(TrackPosition {
                index: 1,
                offset: 50.0,
                in_gap: false
            })
        );
    }

    #[test]
    fn empty_side_has_no_position() {
        let empty = SideTimeline {
            label: "A".into(),
            tracks: vec![],
            total: 0.0,
        };
        assert_eq!(empty.track_at(0.0), None);
    }

    #[test]
    fn clamps_absurd_pregaps() {
        let mut side = side();
        side.tracks[0].pregap = 60.0;
        let timeline = SideTimeline::from_side(&side);
        assert!((timeline.tracks[0].pregap - 10.0).abs() < 1e-9);
    }
}
