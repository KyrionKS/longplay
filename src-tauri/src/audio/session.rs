//! Playback session: loads sides in the background and exposes state for IPC.

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use crate::library::model::{DiscFormat, Rpm, Side};

use super::decode;
use super::engine::Engine;
use super::groove::{GrooveBand, GrooveLayout};
use super::timeline::SideTimeline;

/// Sides are decoded once at this rate, regardless of the device rate. A 96 kHz
/// interface then resamples on playback instead of holding a 1 GB buffer.
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
    issues: Mutex<Vec<String>>,
    error: Mutex<Option<String>>,
    nominal_rpm: AtomicU32,
    current_rpm: AtomicU32,
    album_id: Mutex<String>,
    side_index: AtomicU32,
    format: Mutex<DiscFormat>,
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
        Self {
            engine: Arc::new(Engine::new()),
            token: AtomicU64::new(0),
            loading: AtomicBool::new(false),
            output_rate: AtomicU32::new(48_000),
            timeline: Mutex::new(None),
            issues: Mutex::new(Vec::new()),
            error: Mutex::new(None),
            nominal_rpm: AtomicU32::new((33.333f32).to_bits()),
            current_rpm: AtomicU32::new((33.333f32).to_bits()),
            album_id: Mutex::new(String::new()),
            side_index: AtomicU32::new(0),
            format: Mutex::new(DiscFormat::Lp12),
        }
    }

    pub fn start_output(&self) {
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
        if let Ok(mut slot) = self.format.lock() {
            *slot = format;
        }
        if let Ok(mut slot) = self.album_id.lock() {
            *slot = album_id.to_string();
        }
        self.side_index
            .store(side_index.min(u32::MAX as usize) as u32, Ordering::Relaxed);
        let token = self.token.fetch_add(1, Ordering::SeqCst) + 1;
        self.loading.store(true, Ordering::Relaxed);
        self.engine.pause();
        self.engine.set_side(None);
        if let Ok(mut issues) = self.issues.lock() {
            issues.clear();
        }

        let timeline = SideTimeline::from_side(side);
        let nominal = side.rpm.value();
        store_f32(&self.nominal_rpm, nominal as f32);
        store_f32(&self.current_rpm, nominal as f32);
        self.engine.set_rate(1.0);
        if let Ok(mut slot) = self.timeline.lock() {
            *slot = Some(timeline.clone());
        }

        let side = side.clone();

        std::thread::spawn(move || {
            let (audio, issues) = decode::build_side(&side, &timeline, ENGINE_RATE);
            if session.token.load(Ordering::SeqCst) != token {
                return;
            }
            if let Ok(mut slot) = session.issues.lock() {
                *slot = issues;
            }
            session.engine.set_side(Some(Arc::new(audio)));
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

    pub fn toggle(&self) {
        self.engine.toggle();
    }

    pub fn stop(&self) {
        self.engine.stop();
    }

    pub fn seek(&self, seconds: f64) {
        self.engine.seek_seconds(seconds);
    }

    pub fn set_rpm(&self, rpm: u32) {
        let nominal = f64::from(load_f32(&self.nominal_rpm)).max(1.0);
        let target = f64::from(rpm.clamp(16, 100));
        store_f32(&self.current_rpm, target as f32);
        self.engine.set_rate(target / nominal);
    }

    pub fn state(&self) -> PlaybackState {
        let timeline = self.timeline.lock().ok().and_then(|slot| slot.clone());
        let position = self.engine.position_seconds();
        let duration = timeline
            .as_ref()
            .map(|timeline| timeline.total)
            .unwrap_or_else(|| self.engine.duration_seconds());
        let track = timeline
            .as_ref()
            .and_then(|timeline| timeline.track_at(position));
        let track_span = track.and_then(|position| {
            timeline
                .as_ref()
                .and_then(|timeline| timeline.tracks.get(position.index))
                .cloned()
        });

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

        PlaybackState {
            loaded: self.engine.has_side(),
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
            track_index: track.map(|position| position.index).unwrap_or(0),
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
