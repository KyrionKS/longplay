//! Groove layout: where the tracks sit on the record.
//!
//! Real dimensions: a 12" LP is 302 mm across with a 100 mm label; the music
//! runs from a 146 mm outer groove to 60 mm, with a 5 mm lead-in and a run-out
//! to 54 mm. A 7" single is 175 mm across with an 86 mm label, music from 84 mm
//! to 45 mm. Tracks are separated by a small land (1.6–1.8 mm), the rest of the
//! music area is divided by playing time. Radii are millimetres from the spindle.

use serde::Serialize;

use super::timeline::SideTimeline;
use crate::library::model::{DiscFormat, Rpm};

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum GrooveKind {
    LeadIn,
    Gap,
    Track,
    RunOut,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GrooveBand {
    pub kind: GrooveKind,
    pub track_index: Option<usize>,
    /// Outer radius of the band in mm.
    pub start: f64,
    /// Inner radius of the band in mm.
    pub end: f64,
    /// `start` normalised to the record radius (`0.0` centre, `1.0` edge).
    pub start_norm: f64,
    pub end_norm: f64,
    /// Side time at the outer edge of the band.
    pub start_time: f64,
    /// Side time at the inner edge of the band.
    pub end_time: f64,
}

#[derive(Debug, Clone)]
pub struct GrooveLayout {
    pub record_radius: f64,
    pub bands: Vec<GrooveBand>,
}

struct Dimensions {
    record_radius: f64,
    music_outer: f64,
    music_inner: f64,
    runout_end: f64,
}

fn dimensions(single: bool) -> Dimensions {
    if single {
        Dimensions {
            record_radius: 87.5,
            music_outer: 84.0,
            music_inner: 45.0,
            runout_end: 43.0,
        }
    } else {
        Dimensions {
            record_radius: 151.0,
            music_outer: 146.0,
            music_inner: 60.0,
            runout_end: 54.0,
        }
    }
}

impl GrooveLayout {
    /// Stylus radius (mm) for a side position in seconds.
    pub fn radius_for_time(&self, seconds: f64) -> Option<f64> {
        if self.bands.is_empty() {
            return None;
        }
        let time = seconds.max(0.0);
        for band in &self.bands {
            let span = band.end_time - band.start_time;
            if span > 1e-9 && time >= band.start_time && time <= band.end_time {
                let progress = ((time - band.start_time) / span).clamp(0.0, 1.0);
                return Some(band.start + (band.end - band.start) * progress);
            }
            if time < band.start_time {
                return Some(band.start);
            }
        }
        self.bands.last().map(|band| band.end)
    }

    pub fn for_side(timeline: &SideTimeline, format: DiscFormat, rpm: Rpm) -> Self {
        let single = format == DiscFormat::Single7 || rpm == Rpm::R45;
        let dims = dimensions(single);
        let record_radius = dims.record_radius;
        let music_area = (dims.music_outer - dims.music_inner).max(1.0);

        let total_duration: f64 = timeline
            .tracks
            .iter()
            .map(|track| track.duration.max(0.0))
            .sum();

        // Each track gets a land before it. A real pregap gets a wider land;
        // a gapless boundary gets a hairline so the following track does not
        // drift inwards. The lands are subtracted from the music area.
        let base_land = if single { 0.2 } else { 0.25 };
        let mut gap_widths: Vec<f64> = timeline
            .tracks
            .iter()
            .enumerate()
            .map(|(index, track)| {
                if index == 0 && track.pregap <= 0.01 {
                    0.0
                } else if track.pregap > 0.01 {
                    (0.25 + track.pregap * 0.35).clamp(0.4, 1.4)
                } else {
                    base_land
                }
            })
            .collect();
        let mut gap_total: f64 = gap_widths.iter().sum();
        let gap_budget = music_area * 0.3;
        if gap_total > gap_budget && gap_total > 0.0 {
            let factor = gap_budget / gap_total;
            for gap in &mut gap_widths {
                *gap *= factor;
            }
            gap_total = gap_budget;
        }
        let music_width = (music_area - gap_total).max(0.0);

        let mut cursor = dims.music_outer;
        let mut time_cursor = 0.0;
        let mut bands = vec![GrooveBand {
            kind: GrooveKind::LeadIn,
            track_index: None,
            start: record_radius,
            end: dims.music_outer,
            start_norm: 1.0,
            end_norm: dims.music_outer / record_radius,
            start_time: 0.0,
            end_time: 0.0,
        }];

        for (index, track) in timeline.tracks.iter().enumerate() {
            let gap = gap_widths[index];
            if gap > 0.001 {
                bands.push(GrooveBand {
                    kind: GrooveKind::Gap,
                    track_index: Some(index),
                    start: cursor,
                    end: cursor - gap,
                    start_norm: cursor / record_radius,
                    end_norm: (cursor - gap) / record_radius,
                    start_time: time_cursor,
                    end_time: time_cursor + track.pregap,
                });
                cursor -= gap;
                time_cursor += track.pregap;
            }

            let width = if total_duration > 0.0 {
                music_width * track.duration.max(0.0) / total_duration
            } else {
                0.0
            };
            bands.push(GrooveBand {
                kind: GrooveKind::Track,
                track_index: Some(index),
                start: cursor,
                end: cursor - width,
                start_norm: cursor / record_radius,
                end_norm: (cursor - width) / record_radius,
                start_time: time_cursor,
                end_time: time_cursor + track.duration.max(0.0),
            });
            cursor -= width;
            time_cursor += track.duration.max(0.0);
        }

        bands.push(GrooveBand {
            kind: GrooveKind::RunOut,
            track_index: None,
            start: cursor,
            end: dims.runout_end.min(cursor),
            start_norm: cursor / record_radius,
            end_norm: dims.runout_end.min(cursor) / record_radius,
            start_time: time_cursor,
            end_time: time_cursor,
        });

        Self {
            record_radius,
            bands,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::model::{Side, Track};

    fn side(durations: &[f64], pregaps: &[f64]) -> Side {
        Side {
            label: "A".into(),
            rpm: Rpm::R33,
            matrix: None,
            tracks: durations
                .iter()
                .zip(pregaps)
                .enumerate()
                .map(|(index, (duration, pregap))| Track {
                    id: format!("t{index}"),
                    number: index as u32 + 1,
                    title: format!("Track {index}"),
                    artist: "A".into(),
                    duration: *duration,
                    pregap: *pregap,
                    path: "/x.flac".into(),
                    offset: 0.0,
                    side: "A".into(),
                })
                .collect(),
        }
    }

    fn layout(durations: &[f64], pregaps: &[f64]) -> GrooveLayout {
        let timeline = SideTimeline::from_side(&side(durations, pregaps));
        GrooveLayout::for_side(&timeline, DiscFormat::Lp12, Rpm::R33)
    }

    #[test]
    fn first_track_starts_at_the_outer_music_groove() {
        let layout = layout(&[300.0, 180.0], &[0.0, 0.0]);
        let lead_in = &layout.bands[0];
        assert_eq!(lead_in.kind, GrooveKind::LeadIn);
        assert_eq!(lead_in.start, 151.0);
        assert_eq!(lead_in.end, 146.0);

        let first_track = layout
            .bands
            .iter()
            .find(|band| band.kind == GrooveKind::Track)
            .unwrap();
        assert_eq!(first_track.start, 146.0);
    }

    #[test]
    fn bands_are_contiguous_from_edge_to_runout() {
        let layout = layout(&[300.0, 180.0, 240.0], &[1.5, 1.5, 1.5]);
        assert_eq!(layout.bands.first().unwrap().start, 151.0);
        for pair in layout.bands.windows(2) {
            assert!((pair[0].end - pair[1].start).abs() < 1e-6);
            assert!(pair[0].start >= pair[0].end);
        }
        let last = layout.bands.last().unwrap();
        assert_eq!(last.kind, GrooveKind::RunOut);
        assert!((last.end - 54.0).abs() < 1e-9);
    }

    #[test]
    fn track_widths_follow_durations() {
        let layout = layout(&[300.0, 150.0], &[0.0, 0.0]);
        let tracks: Vec<_> = layout
            .bands
            .iter()
            .filter(|band| band.kind == GrooveKind::Track)
            .collect();
        assert_eq!(tracks.len(), 2);
        let first = tracks[0].start - tracks[0].end;
        let second = tracks[1].start - tracks[1].end;
        assert!((first / second - 2.0).abs() < 0.05, "{first} vs {second}");
    }

    #[test]
    fn lands_follow_the_real_pauses() {
        let layout = layout(&[300.0, 250.0, 200.0], &[0.0, 1.5, 0.0]);
        let lands: Vec<_> = layout
            .bands
            .iter()
            .filter(|band| band.kind == GrooveKind::Gap)
            .collect();
        assert_eq!(lands.len(), 2);
        let pause = lands[0].start - lands[0].end;
        let gapless = lands[1].start - lands[1].end;
        assert!((0.6..=1.0).contains(&pause), "pause land {pause}");
        assert!(gapless <= 0.3, "gapless land {gapless}");
    }

    #[test]
    fn track_bands_start_exactly_at_the_audio_tracks() {
        let side = side(&[300.0, 180.0, 240.0], &[1.5, 1.5, 1.5]);
        let timeline = SideTimeline::from_side(&side);
        let layout = GrooveLayout::for_side(&timeline, DiscFormat::Lp12, Rpm::R33);

        for (index, track) in timeline.tracks.iter().enumerate() {
            let band = layout
                .bands
                .iter()
                .find(|band| band.kind == GrooveKind::Track && band.track_index == Some(index))
                .unwrap_or_else(|| panic!("track band {index} missing"));
            assert!(
                (band.start_time - track.start).abs() < 1e-9,
                "track {index}: band starts at {} but audio at {}",
                band.start_time,
                track.start
            );
            assert!(
                (band.end_time - (track.start + track.duration)).abs() < 1e-9,
                "track {index}: band ends at {} but audio at {}",
                band.end_time,
                track.start + track.duration
            );
            let radius = layout.radius_for_time(track.start).unwrap();
            assert!((radius - band.start).abs() < 1e-6);
        }
    }

    #[test]
    fn maps_time_to_radius_monotonically() {
        let layout = layout(&[300.0, 150.0], &[1.5, 1.5]);
        let mut previous = f64::MAX;
        for step in 0..=100 {
            let time = 453.0 * step as f64 / 100.0;
            let radius = layout.radius_for_time(time).unwrap();
            assert!(radius <= previous + 1e-9, "radius must shrink over time");
            previous = radius;
        }
        assert!((layout.radius_for_time(0.0).unwrap() - 146.0).abs() < 1e-9);
        assert!(layout.radius_for_time(453.0).unwrap() <= 60.0 + 1e-6);
    }

    #[test]
    fn singles_use_the_7_inch_dimensions() {
        let timeline = SideTimeline::from_side(&side(&[120.0, 130.0], &[0.0, 0.0]));
        let layout = GrooveLayout::for_side(&timeline, DiscFormat::Single7, Rpm::R45);
        assert_eq!(layout.record_radius, 87.5);
        assert_eq!(layout.bands.first().unwrap().start, 87.5);
        assert_eq!(layout.bands.first().unwrap().end, 84.0);
        let last = layout.bands.last().unwrap();
        assert!((last.end - 43.0).abs() < 1e-9);
    }

    #[test]
    fn empty_side_still_has_lead_in_and_runout() {
        let layout = layout(&[], &[]);
        assert_eq!(layout.bands.len(), 2);
        assert_eq!(layout.bands[0].kind, GrooveKind::LeadIn);
        assert_eq!(layout.bands[1].kind, GrooveKind::RunOut);
    }

    #[test]
    fn many_tracks_do_not_overflow_the_groove_area() {
        let durations = vec![30.0; 40];
        let pregaps = vec![1.5; 40];
        let layout = layout(&durations, &pregaps);
        for band in &layout.bands {
            assert!(band.start >= band.end);
            assert!(band.end >= 54.0 - 1e-9);
        }
    }
}
