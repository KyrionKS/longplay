//! Groove layout: where the tracks sit on the record.
//!
//! Radii are in millimetres, measured from the spindle. The playable area is
//! divided by time, with a small minimum land between tracks so the bands stay
//! readable. The same layout will drive the tonearm in phase 3.

use serde::Serialize;

use crate::library::model::{DiscFormat, Rpm};
use crate::playback::TonearmGeometry;

use super::timeline::SideTimeline;

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
        let geometry = if single {
            TonearmGeometry::SINGLE7
        } else {
            TonearmGeometry::LP12
        };
        let record_radius = if single { 87.5 } else { 151.0 };
        let outer = geometry.outer_radius;
        let inner = geometry.inner_radius;
        let label_radius = record_radius * 0.36;
        // Real records have a lead-in groove; a slightly wider one keeps the
        // cartridge visibly on the record at the first track.
        let music_outer = outer - if single { 13.0 } else { 14.0 };
        let available = (music_outer - inner).max(1.0);
        let min_gap = if single { 2.2 } else { 1.6 };

        let total_duration: f64 = timeline
            .tracks
            .iter()
            .map(|track| track.duration.max(0.0))
            .sum();

        // A pregap gets at least a visible land; tracks without a pregap still
        // get a thin separator so the band structure reads like a record.
        let mut gaps: Vec<f64> = timeline
            .tracks
            .iter()
            .enumerate()
            .map(|(index, track)| {
                if track.pregap > 0.01 {
                    (available * track.pregap / timeline.total.max(1.0)).max(min_gap)
                } else if index > 0 {
                    min_gap * 0.9
                } else {
                    0.0
                }
            })
            .collect();

        let mut gap_sum: f64 = gaps.iter().sum();
        let gap_budget = available * 0.55;
        if gap_sum > gap_budget && gap_sum > 0.0 {
            let factor = gap_budget / gap_sum;
            for gap in &mut gaps {
                *gap *= factor;
            }
            gap_sum = gap_budget;
        }

        let music_width = (available - gap_sum).max(0.0);
        let mut cursor = music_outer;
        let mut time_cursor = 0.0;
        let mut bands = vec![GrooveBand {
            kind: GrooveKind::LeadIn,
            track_index: None,
            start: record_radius,
            end: music_outer,
            start_norm: 1.0,
            end_norm: music_outer / record_radius,
            start_time: 0.0,
            end_time: 0.0,
        }];

        for (index, track) in timeline.tracks.iter().enumerate() {
            let gap = gaps[index];
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
            }
            time_cursor += track.pregap;

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
                end_time: time_cursor + track.duration,
            });
            cursor -= width;
            time_cursor += track.duration;
        }

        let runout_end = (inner - 8.0).max(label_radius).min(cursor);
        bands.push(GrooveBand {
            kind: GrooveKind::RunOut,
            track_index: None,
            start: cursor,
            end: runout_end,
            start_norm: cursor / record_radius,
            end_norm: runout_end / record_radius,
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
    fn maps_time_to_radius_monotonically() {
        let layout = layout(&[300.0, 150.0], &[1.5, 1.5]);
        let mut previous = f64::MAX;
        for step in 0..=100 {
            let time = 453.0 * step as f64 / 100.0;
            let radius = layout.radius_for_time(time).unwrap();
            assert!(radius <= previous + 1e-9, "radius must shrink over time");
            previous = radius;
        }
        let start = layout.radius_for_time(0.0).unwrap();
        let end = layout.radius_for_time(453.0).unwrap();
        assert!((start - 132.0).abs() < 1e-9);
        assert!(end <= 60.0 + 1e-6);
    }

    #[test]
    fn band_times_cover_the_side() {
        let layout = layout(&[300.0, 150.0], &[1.5, 1.5]);
        let first = layout.bands.first().unwrap();
        let last = layout.bands.last().unwrap();
        assert_eq!(first.start_time, 0.0);
        assert!((last.end_time - 453.0).abs() < 1e-9);
        for pair in layout.bands.windows(2) {
            assert!(pair[0].end_time <= pair[1].start_time + 1e-9);
        }
    }

    #[test]
    fn bands_are_contiguous_from_edge_to_runout() {
        let layout = layout(&[300.0, 180.0, 240.0], &[1.5, 1.5, 1.5]);
        assert_eq!(layout.bands.first().unwrap().kind, GrooveKind::LeadIn);
        assert_eq!(layout.bands.first().unwrap().start, 151.0);
        assert!((layout.bands.first().unwrap().end - 132.0).abs() < 1e-9);
        for pair in layout.bands.windows(2) {
            assert!((pair[0].end - pair[1].start).abs() < 1e-6);
            assert!(pair[0].start >= pair[0].end);
        }
        let last = layout.bands.last().unwrap();
        assert_eq!(last.kind, GrooveKind::RunOut);
        assert!((last.end - 151.0 * 0.36).abs() < 1e-6);
    }

    #[test]
    fn track_widths_follow_durations() {
        let layout = layout(&[300.0, 150.0], &[1.5, 1.5]);
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
    fn gaps_are_at_least_visible() {
        let layout = layout(&[600.0, 600.0], &[0.0, 0.0]);
        for band in layout
            .bands
            .iter()
            .filter(|band| band.kind == GrooveKind::Gap)
        {
            assert!(band.start - band.end >= 0.45, "gap too small");
        }
    }

    #[test]
    fn singles_use_the_smaller_record() {
        let timeline = SideTimeline::from_side(&side(&[120.0, 130.0], &[0.0, 0.0]));
        let layout = GrooveLayout::for_side(&timeline, DiscFormat::Single7, Rpm::R45);
        assert_eq!(layout.record_radius, 87.5);
        assert_eq!(layout.bands.first().unwrap().start, 87.5);
        assert!((layout.bands.last().unwrap().end - 87.5 * 0.36).abs() < 1e-6);
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
            assert!(band.end >= 151.0 * 0.36 - 1e-9);
        }
    }
}
