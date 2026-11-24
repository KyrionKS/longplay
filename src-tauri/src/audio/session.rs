//! Playback session: loads one track at a time, prefetches the next and exposes
//! state for IPC.
//!
//! A whole side is never held in memory. The conductor thread watches the
//! engine and prepares the following track in the background, so albums still
//! play across track boundaries even when the files live on a NAS.

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::library::model::{DiscFormat, Rpm, Side};

use super::decode;
use super::engine::Engine;
use super::groove::{GrooveBand, GrooveLayout};
use super::timeline::SideTimeline;

/// Tracks are decoded at this rate, regardless of the device rate.
const ENGINE_RATE: u32 = 48_000;

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackState {
    pub loaded: bool,
    pub loading: bool,
    pub playing: bool,
    pub position: f64,
    pub duration: f64,
    pub side: String,
    pub side_index: usize,
    pub album_id: String,
    pub rpm: String,
    pub grooves: Vec<GrooveBand>,
    pub record_radius: f64,
    pub arm_angle: f64,
    pub stylus_radius: f64,
    pub pivot_distance: f64,
    pub arm_length: f64,
    pub track_index: usize,
    pub track_number: u32,
    pub track_title: String,
    pub issues: Vec<String>,
    pub error: Option<String>,
}

pub struct Session {
    engine: Arc<Engine>,
    token: AtomicU64,
    loading: AtomicBool,
    output_rate: AtomicU32,
    timeline: Mutex<Option<SideTimeline>>,
    side: Mutex<Option<Side>>,
    issues: Mutex<Vec<String>>,
    error: Mutex<Option<String>>,
    nominal_rpm: AtomicU32,
    current_rpm: AtomicU32,
    album_id: Mutex<String>,
    side_index: AtomicU32,
    format: Mutex<DiscFormat>,
    prefetching: AtomicBool,
}

fn store_f32(atomic: &AtomicU32, value: f32) {
    atomic.store(value.to_bits(), Ordering::Relaxed);
}

fn load_f32(atomic: &AtomicU32) -> f32 {
    f32::from_bits(atomic.load(Ordering::Relaxed))
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

impl Session {
    pub fn new() -> Self {
        let session = Self {
            engine: Arc::new(Engine::new()),
            token: AtomicU64::new(0),
            loading: AtomicBool::new(false),
            output_rate: AtomicU32::new(48_000),
            timeline: Mutex::new(None),
            side: Mutex::new(None),
            issues: Mutex::new(Vec::new()),
            error: Mutex::new(None),
            nominal_rpm: AtomicU32::new((33.333f32).to_bits()),
            current_rpm: AtomicU32::new((33.333f32).to_bits()),
            album_id: Mutex::new(String::new()),
            side_index: AtomicU32::new(0),
            format: Mutex::new(DiscFormat::Lp12),
            prefetching: AtomicBool::new(false),
        };
        session.engine.set_needle_pre_roll(true);
        session
    }

    pub fn start_output(self: &Arc<Self>) {
        match super::output::start(self.engine.clone()) {
            Ok(rate) => {
                self.output_rate.store(rate, Ordering::Relaxed);
                eprintln!("Audio-Ausgabe bereit: {rate} Hz");
            }
            Err(error) => {
                eprintln!("Audio-Ausgabe fehlgeschlagen: {error}");
                if let Ok(mut slot) = self.error.lock() {
                    *slot = Some(error);
                }
            }
        }
        self.start_conductor();
    }

    fn start_conductor(self: &Arc<Self>) {
        let session = Arc::clone(self);
        std::thread::spawn(move || {
            loop {
                session.tick();
                std::thread::sleep(Duration::from_millis(120));
            }
        });
    }

    fn tick(self: &Arc<Self>) {
        self.engine.drain_retired();
        if self.loading.load(Ordering::Relaxed) {
            return;
        }
        if !self.engine.has_track() || self.engine.has_next() {
            return;
        }
        let Some(timeline) = self.timeline.lock().ok().and_then(|slot| slot.clone()) else {
            return;
        };
        let next = self.engine.track_index() + 1;
        if next >= timeline.tracks.len() {
            return;
        }
        self.schedule_prefetch(next);
    }

    fn schedule_prefetch(self: &Arc<Self>, index: usize) {
        if self.prefetching.swap(true, Ordering::SeqCst) {
            return;
        }

        let session = Arc::clone(self);
        let token = self.token.load(Ordering::SeqCst);
        std::thread::spawn(move || {
            let side = session.side.lock().ok().and_then(|slot| slot.clone());
            let timeline = session.timeline.lock().ok().and_then(|slot| slot.clone());
            if let (Some(side), Some(timeline)) = (side, timeline) {
                let (track, issues) = decode::build_track(&side, &timeline, index, ENGINE_RATE);
                let current = session.engine.track_index();
                if session.token.load(Ordering::SeqCst) == token
                    && current + 1 == index
                    && !session.engine.has_next()
                {
                    if let Ok(mut slot) = session.issues.lock() {
                        slot.extend(issues);
                    }
                    session.engine.queue_next(Arc::new(track));
                }
            }
            session.prefetching.store(false, Ordering::SeqCst);
        });
    }

    pub fn load(
        self: &Arc<Self>,
        album_id: &str,
        side_index: usize,
        side: &Side,
        format: DiscFormat,
        autoplay: bool,
    ) {
        let session = Arc::clone(self);
        let token = self.token.fetch_add(1, Ordering::SeqCst) + 1;
        self.loading.store(true, Ordering::Relaxed);
        self.engine.pause();
        self.engine.clear();
        if let Ok(mut issues) = self.issues.lock() {
            issues.clear();
        }
        if let Ok(mut slot) = self.format.lock() {
            *slot = format;
        }
        if let Ok(mut slot) = self.album_id.lock() {
            *slot = album_id.to_string();
        }
        self.side_index
            .store(side_index.min(u32::MAX as usize) as u32, Ordering::Relaxed);

        let timeline = SideTimeline::from_side(side);
        let nominal = side.rpm.value();
        store_f32(&self.nominal_rpm, nominal as f32);
        store_f32(&self.current_rpm, nominal as f32);
        self.engine.set_rate(1.0);
        if let Ok(mut slot) = self.timeline.lock() {
            *slot = Some(timeline.clone());
        }
        if let Ok(mut slot) = self.side.lock() {
            *slot = Some(side.clone());
        }

        let side = side.clone();
        std::thread::spawn(move || {
            let (track, issues) = decode::build_track(&side, &timeline, 0, ENGINE_RATE);
            if session.token.load(Ordering::SeqCst) != token {
                return;
            }
            if let Ok(mut slot) = session.issues.lock() {
                *slot = issues;
            }
            session.engine.load_track(Arc::new(track), 0.0);
            session.loading.store(false, Ordering::Relaxed);
            if autoplay {
                session.engine.play();
            }
        });
    }

    pub fn play(&self) {
        self.engine.play();
    }

    pub fn pause(&self) {
        self.engine.pause();
    }

    pub fn toggle(self: &Arc<Self>) {
        if self.loading.load(Ordering::Relaxed) || !self.engine.has_track() {
            return;
        }
        let Some(timeline) = self.timeline.lock().ok().and_then(|slot| slot.clone()) else {
            return;
        };
        let at_end = self.engine.side_position_seconds() >= timeline.total - 0.05;
        if at_end && !self.engine.is_playing() {
            self.restart_side();
            return;
        }
        self.engine.toggle();
    }

    fn restart_side(self: &Arc<Self>) {
        let side = self.side.lock().ok().and_then(|slot| slot.clone());
        let format = self
            .format
            .lock()
            .map(|slot| *slot)
            .unwrap_or(DiscFormat::Lp12);
        let album_id = self
            .album_id
            .lock()
            .map(|slot| slot.clone())
            .unwrap_or_default();
        let side_index = self.side_index.load(Ordering::Relaxed) as usize;
        if let Some(side) = side {
            self.load(&album_id, side_index, &side, format, true);
        }
    }

    pub fn stop(&self) {
        self.engine.stop();
    }

    pub fn seek(self: &Arc<Self>, seconds: f64) {
        self.seek_internal(seconds, false);
    }

    /// Cueing: seek and start playing once the target track is ready.
    pub fn cue(self: &Arc<Self>, seconds: f64) {
        self.seek_internal(seconds, true);
    }

    fn seek_internal(self: &Arc<Self>, seconds: f64, play_after: bool) {
        let Some(timeline) = self.timeline.lock().ok().and_then(|slot| slot.clone()) else {
            return;
        };
        let Some(target) = timeline.track_at(seconds) else {
            return;
        };
        let Some(span) = timeline.tracks.get(target.index).cloned() else {
            return;
        };

        if self.engine.has_track()
            && !self.loading.load(Ordering::Relaxed)
            && self.engine.track_index() == target.index
        {
            let local = if target.in_gap {
                target.offset
            } else {
                span.pregap + target.offset
            };
            self.engine.seek_local_seconds(local);
            if play_after {
                self.engine.request_cue_drop();
                self.engine.play();
            }
            return;
        }

        let Some(side) = self.side.lock().ok().and_then(|slot| slot.clone()) else {
            return;
        };
        let was_playing = play_after || self.engine.is_playing();
        let local = if target.in_gap {
            target.offset
        } else {
            span.pregap + target.offset
        };

        // The next track is usually already decoded; a cue into it is instant
        // even when the files live on a NAS.
        if let Some(track) = self.engine.take_next(target.index) {
            self.engine.load_track(track, local);
            if was_playing {
                if play_after {
                    self.engine.request_cue_drop();
                } else {
                    self.engine.suppress_next_pre_roll();
                }
                self.engine.play();
            }
            return;
        }

        let token = self.token.fetch_add(1, Ordering::SeqCst) + 1;
        self.loading.store(true, Ordering::Relaxed);
        self.engine.pause();
        self.engine.clear_next();

        let session = Arc::clone(self);
        std::thread::spawn(move || {
            let (track, issues) = decode::build_track(&side, &timeline, target.index, ENGINE_RATE);
            if session.token.load(Ordering::SeqCst) != token {
                return;
            }
            if let Ok(mut slot) = session.issues.lock() {
                slot.extend(issues);
            }
            session.engine.load_track(Arc::new(track), local);
            session.loading.store(false, Ordering::Relaxed);
            if was_playing {
                if play_after {
                    session.engine.request_cue_drop();
                } else {
                    session.engine.suppress_next_pre_roll();
                }
                session.engine.play();
            }
        });
    }

    pub fn set_rpm(&self, rpm: u32) {
        let nominal = f64::from(load_f32(&self.nominal_rpm)).max(1.0);
        let target = f64::from(rpm.clamp(16, 100));
        store_f32(&self.current_rpm, target as f32);
        self.engine.set_rate(target / nominal);
    }

    pub fn state(&self) -> PlaybackState {
        let timeline = self.timeline.lock().ok().and_then(|slot| slot.clone());
        let position = self.engine.side_position_seconds();
        let duration = timeline
            .as_ref()
            .map(|timeline| timeline.total)
            .unwrap_or_else(|| self.engine.duration_seconds());
        let track_index = if self.engine.has_track() {
            self.engine.track_index()
        } else {
            0
        };
        let track_span = timeline
            .as_ref()
            .and_then(|timeline| timeline.tracks.get(track_index))
            .cloned();

        let format = self
            .format
            .lock()
            .map(|format| *format)
            .unwrap_or(DiscFormat::Lp12);
        let rpm = if load_f32(&self.current_rpm) >= 40.0 {
            Rpm::R45
        } else {
            Rpm::R33
        };
        let layout = timeline
            .as_ref()
            .map(|timeline| GrooveLayout::for_side(timeline, format, rpm));
        let single = format == DiscFormat::Single7 || rpm == Rpm::R45;
        let geometry = if single {
            crate::playback::TonearmGeometry::SINGLE7
        } else {
            crate::playback::TonearmGeometry::LP12
        };
        let stylus_radius = layout
            .as_ref()
            .and_then(|layout| layout.radius_for_time(position))
            .unwrap_or(geometry.outer_radius);
        let arm_angle = geometry.angle_for_radius(stylus_radius);

        PlaybackState {
            loaded: self.engine.has_track(),
            loading: self.loading.load(Ordering::Relaxed),
            playing: self.engine.is_playing(),
            position,
            duration,
            side: timeline
                .as_ref()
                .map(|timeline| timeline.label.clone())
                .unwrap_or_default(),
            side_index: self.side_index.load(Ordering::Relaxed) as usize,
            album_id: self
                .album_id
                .lock()
                .map(|id| id.clone())
                .unwrap_or_default(),
            rpm: format!("{}", if rpm == Rpm::R45 { 45 } else { 33 }),
            grooves: layout
                .as_ref()
                .map(|layout| layout.bands.clone())
                .unwrap_or_default(),
            record_radius: layout.map(|layout| layout.record_radius).unwrap_or(151.0),
            arm_angle,
            stylus_radius,
            pivot_distance: geometry.pivot_distance,
            arm_length: geometry.effective_length,
            track_index,
            track_number: track_span.as_ref().map(|track| track.number).unwrap_or(0),
            track_title: track_span.map(|track| track.title).unwrap_or_default(),
            issues: self
                .issues
                .lock()
                .map(|issues| issues.clone())
                .unwrap_or_default(),
            error: self.error.lock().ok().and_then(|error| error.clone()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::decode::TrackAudio;

    #[cfg(target_os = "linux")]
    #[test]
    fn cue_into_the_prefetched_next_track_is_immediate() {
        use crate::library::model::{Rpm, Side, Track};
        use std::fs;
        use std::path::PathBuf;
        use std::time::{Duration, Instant};

        fn write_wav(path: &std::path::Path, seconds: f64) {
            let rate = 8_000u32;
            let frames = (seconds * f64::from(rate)) as u32;
            let data_len = frames * 4;
            let mut out = Vec::new();
            out.extend_from_slice(b"RIFF");
            out.extend_from_slice(&(36 + data_len).to_le_bytes());
            out.extend_from_slice(b"WAVEfmt ");
            out.extend_from_slice(&16u32.to_le_bytes());
            out.extend_from_slice(&1u16.to_le_bytes());
            out.extend_from_slice(&2u16.to_le_bytes());
            out.extend_from_slice(&rate.to_le_bytes());
            out.extend_from_slice(&(rate * 4).to_le_bytes());
            out.extend_from_slice(&4u16.to_le_bytes());
            out.extend_from_slice(&16u16.to_le_bytes());
            out.extend_from_slice(b"data");
            out.extend_from_slice(&data_len.to_le_bytes());
            out.resize(out.len() + data_len as usize, 0);
            fs::write(path, out).unwrap();
        }

        fn wait_until(mut condition: impl FnMut() -> bool) {
            let deadline = Instant::now() + Duration::from_secs(4);
            while Instant::now() < deadline {
                if condition() {
                    return;
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            panic!("condition not reached in time");
        }

        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir: PathBuf =
            std::env::temp_dir().join(format!("longplay-cue-{}-{stamp}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("tone.wav");
        write_wav(&path, 2.0);
        let track = |index: usize| Track {
            id: format!("t{index}"),
            number: index as u32 + 1,
            title: format!("Track {}", index + 1),
            artist: "Test".into(),
            duration: 2.0,
            pregap: 0.0,
            path: path.to_string_lossy().into_owned(),
            offset: 0.0,
            side: "A".into(),
        };
        let side = Side {
            label: "A".into(),
            rpm: Rpm::R33,
            matrix: None,
            tracks: vec![track(0), track(1)],
        };

        let session = Arc::new(Session::new());
        session.engine.set_runout_seconds(0.3);
        session.start_output();
        session.load("album", 0, &side, DiscFormat::Lp12, true);
        wait_until(|| session.state().loaded);

        // Give the conductor time to prefetch the next track.
        std::thread::sleep(Duration::from_millis(400));
        session.cue(2.1);

        wait_until(|| {
            let state = session.state();
            state.track_index == 1 && state.playing
        });
        let state = session.state();
        assert!(!state.loading, "prefetched cue must not block");
        assert!(state.position >= 2.05, "position {}", state.position);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn loads_state_before_any_track() {
        let session = Session::new();
        let state = session.state();
        assert!(!state.loaded);
        assert!(!state.playing);
        assert_eq!(state.position, 0.0);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn plays_advances_and_seeks_between_tracks() {
        use crate::library::model::{Rpm, Side, Track};
        use std::fs;
        use std::path::{Path, PathBuf};
        use std::time::{Duration, Instant};

        fn write_wav(path: &Path, seconds: f64) {
            let rate = 8_000u32;
            let frames = (seconds * f64::from(rate)) as u32;
            let data_len = frames * 4;
            let mut out = Vec::new();
            out.extend_from_slice(b"RIFF");
            out.extend_from_slice(&(36 + data_len).to_le_bytes());
            out.extend_from_slice(b"WAVEfmt ");
            out.extend_from_slice(&16u32.to_le_bytes());
            out.extend_from_slice(&1u16.to_le_bytes());
            out.extend_from_slice(&2u16.to_le_bytes());
            out.extend_from_slice(&rate.to_le_bytes());
            out.extend_from_slice(&(rate * 4).to_le_bytes());
            out.extend_from_slice(&4u16.to_le_bytes());
            out.extend_from_slice(&16u16.to_le_bytes());
            out.extend_from_slice(b"data");
            out.extend_from_slice(&data_len.to_le_bytes());
            for frame in 0..frames {
                let value = ((frame as f32 / rate as f32) * 440.0 * std::f32::consts::TAU).sin();
                let pcm = (value * 8_000.0) as i16;
                out.extend_from_slice(&pcm.to_le_bytes());
                out.extend_from_slice(&pcm.to_le_bytes());
            }
            fs::write(path, out).unwrap();
        }

        fn wait_until(mut condition: impl FnMut() -> bool) {
            let deadline = Instant::now() + Duration::from_secs(6);
            while Instant::now() < deadline {
                if condition() {
                    return;
                }
                std::thread::sleep(Duration::from_millis(25));
            }
            panic!("condition not reached in time");
        }

        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir: PathBuf =
            std::env::temp_dir().join(format!("longplay-session-{}-{stamp}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("tone.wav");
        write_wav(&path, 1.0);
        let track = |index: usize| Track {
            id: format!("t{index}"),
            number: index as u32 + 1,
            title: format!("Track {}", index + 1),
            artist: "Test".into(),
            duration: 1.0,
            pregap: 0.0,
            path: path.to_string_lossy().into_owned(),
            offset: 0.0,
            side: "A".into(),
        };
        let side = Side {
            label: "A".into(),
            rpm: Rpm::R33,
            matrix: None,
            tracks: vec![track(0), track(1)],
        };

        let session = Arc::new(Session::new());
        session.engine.set_runout_seconds(0.3);
        session.start_output();
        session.load("album", 0, &side, DiscFormat::Lp12, true);

        wait_until(|| session.state().loaded);
        let first = session.state();
        assert!(
            (first.stylus_radius - 146.0).abs() < 1.0,
            "first groove at {} mm",
            first.stylus_radius
        );
        wait_until(|| session.state().position > 1.15);
        let state = session.state();
        assert_eq!(state.track_index, 1, "did not advance to the next track");
        assert!(state.playing);

        wait_until(|| !session.state().playing);
        let state = session.state();
        assert!(state.position >= 1.9, "position {}", state.position);

        session.seek(0.2);
        wait_until(|| session.state().track_index == 0 && session.state().position >= 0.15);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn track_audio_is_not_cloned_by_the_queue() {
        // The queue only keeps Arc handles, so the samples live exactly once.
        let track = Arc::new(TrackAudio {
            samples: vec![0.0; 8],
            rate: 48_000,
            duration: 0.0,
            pregap: 0.0,
            track_index: 0,
            side_start: 0.0,
        });
        let engine = Engine::new();
        engine.load_track(track.clone(), 0.0);
        engine.queue_next(track);
        assert!(engine.has_next());
    }
}
