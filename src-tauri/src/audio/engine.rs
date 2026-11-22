//! Playback engine: turns a loaded side into stereo output.
//!
//! The engine is callback-safe: it only touches atomics, an `ArcSwapOption` and
//! the caller-owned [`DspState`]. No locks, no allocations.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};

use arc_swap::ArcSwapOption;

use super::noise::VinylNoise;
use super::resample::read_interleaved;

use super::decode::SideAudio;

pub struct DspState {
    noise: VinylNoise,
    lowpass: [f32; 2],
    time: f64,
    was_playing: bool,
}

impl Default for DspState {
    fn default() -> Self {
        Self::new()
    }
}

impl DspState {
    pub fn new() -> Self {
        Self {
            noise: VinylNoise::new(0x51f0_2c3a),
            lowpass: [0.0; 2],
            time: 0.0,
            was_playing: false,
        }
    }
}

pub struct Engine {
    side: ArcSwapOption<SideAudio>,
    playing: AtomicBool,
    position: AtomicU64,
    duration: AtomicU64,
    rate: AtomicU32,
    volume: AtomicU32,
    character: AtomicU32,
}

fn store_f32(atomic: &AtomicU32, value: f32) {
    atomic.store(value.to_bits(), Ordering::Relaxed);
}

fn load_f32(atomic: &AtomicU32) -> f32 {
    f32::from_bits(atomic.load(Ordering::Relaxed))
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine {
    pub fn new() -> Self {
        Self {
            side: ArcSwapOption::empty(),
            playing: AtomicBool::new(false),
            position: AtomicU64::new(0),
            duration: AtomicU64::new(0),
            rate: AtomicU32::new(1.0f32.to_bits()),
            volume: AtomicU32::new(0.9f32.to_bits()),
            character: AtomicU32::new(0.3f32.to_bits()),
        }
    }

    pub fn set_side(&self, side: Option<Arc<SideAudio>>) {
        self.playing.store(false, Ordering::Relaxed);
        self.position.store(0, Ordering::Relaxed);
        let duration = side
            .as_ref()
            .map(|audio| (audio.duration * f64::from(audio.rate)) as u64)
            .unwrap_or(0);
        self.duration.store(duration, Ordering::Relaxed);
        self.side.store(side);
    }

    pub fn has_side(&self) -> bool {
        self.side.load().is_some()
    }

    pub fn play(&self) {
        let duration = self.duration.load(Ordering::Relaxed);
        if duration == 0 {
            return;
        }
        if self.position.load(Ordering::Relaxed) >= duration {
            self.position.store(0, Ordering::Relaxed);
        }
        self.playing.store(true, Ordering::Relaxed);
    }

    pub fn pause(&self) {
        self.playing.store(false, Ordering::Relaxed);
    }

    pub fn toggle(&self) {
        if self.playing.load(Ordering::Relaxed) {
            self.pause();
        } else {
            self.play();
        }
    }

    pub fn stop(&self) {
        self.playing.store(false, Ordering::Relaxed);
        self.position.store(0, Ordering::Relaxed);
    }

    pub fn is_playing(&self) -> bool {
        self.playing.load(Ordering::Relaxed)
    }

    pub fn position_frames(&self) -> u64 {
        self.position.load(Ordering::Relaxed)
    }

    pub fn duration_seconds(&self) -> f64 {
        let rate = self
            .side
            .load()
            .as_ref()
            .map(|audio| audio.rate)
            .unwrap_or(48_000);
        self.duration.load(Ordering::Relaxed) as f64 / f64::from(rate)
    }

    pub fn position_seconds(&self) -> f64 {
        let rate = self
            .side
            .load()
            .as_ref()
            .map(|audio| audio.rate)
            .unwrap_or(48_000);
        self.position_frames() as f64 / f64::from(rate)
    }

    pub fn seek_seconds(&self, seconds: f64) {
        let Some(side) = self.side.load_full() else {
            return;
        };
        let max = self.duration.load(Ordering::Relaxed) as f64;
        let frames = (seconds.max(0.0) * f64::from(side.rate)).clamp(0.0, max);
        self.position.store(frames as u64, Ordering::Relaxed);
        if frames >= max {
            self.playing.store(false, Ordering::Relaxed);
        }
    }

    pub fn set_rate(&self, rate: f64) {
        store_f32(&self.rate, rate.clamp(-3.0, 3.0) as f32);
    }

    pub fn set_volume(&self, volume: f64) {
        store_f32(&self.volume, volume.clamp(0.0, 1.5) as f32);
    }

    pub fn set_character(&self, character: f64) {
        store_f32(&self.character, character.clamp(0.0, 1.0) as f32);
    }

    pub fn character(&self) -> f32 {
        load_f32(&self.character)
    }

    /// Renders one device buffer. `out` is interleaved stereo f32.
    pub fn process(&self, out: &mut [f32], device_rate: u32, dsp: &mut DspState) {
        out.fill(0.0);
        if out.len() < 2 {
            return;
        }

        let Some(side) = self.side.load_full() else {
            dsp.was_playing = false;
            return;
        };
        if !self.playing.load(Ordering::Relaxed) {
            dsp.was_playing = false;
            return;
        }
        if side.rate == 0 || side.samples.len() < 8 {
            self.playing.store(false, Ordering::Relaxed);
            return;
        }

        if !dsp.was_playing {
            dsp.noise.trigger_drop();
            dsp.was_playing = true;
        }

        let frames = side.samples.len() / 2;
        let mut position = self.position.load(Ordering::Relaxed) as f64;
        let base_rate = f64::from(load_f32(&self.rate))
            * (f64::from(side.rate) / f64::from(device_rate.max(1)));
        let character = load_f32(&self.character);
        let volume = load_f32(&self.volume);
        let sample_rate = f64::from(device_rate.max(1));
        let mut frame_out = [0.0f32; 2];
        let mut finished = false;

        for frame in out.as_chunks_mut::<2>().0 {
            if position >= (frames.saturating_sub(2)) as f64 {
                finished = true;
                break;
            }

            let wow = 1.0
                + f64::from(character)
                    * (0.0015 * (std::f64::consts::TAU * 0.55 * dsp.time).sin()
                        + 0.0005 * (std::f64::consts::TAU * 6.3 * dsp.time).sin());
            let rate = base_rate * wow;

            read_interleaved(&side.samples, frames, 2, position, &mut frame_out);

            // Inner-groove loss: the closer to the label, the duller.
            let progress = (position / frames as f64) as f32;
            let cutoff = 1.0 - 0.55 * progress * character;
            dsp.lowpass[0] += (frame_out[0] - dsp.lowpass[0]) * cutoff;
            dsp.lowpass[1] += (frame_out[1] - dsp.lowpass[1]) * cutoff;
            let mut left = dsp.lowpass[0];
            let mut right = dsp.lowpass[1];

            // Slight channel bleed, as on a real cartridge.
            let crosstalk = 0.03 * character;
            let left_bleed = left + crosstalk * right;
            let right_bleed = right + crosstalk * left;
            left = left_bleed;
            right = right_bleed;

            let (noise_left, noise_right) = dsp.noise.process(character, device_rate, 1.0);
            frame[0] = soft_clip((left + noise_left) * volume);
            frame[1] = soft_clip((right + noise_right) * volume);

            position += rate;
            dsp.time += 1.0 / sample_rate;
        }

        if finished {
            self.position
                .store(self.duration.load(Ordering::Relaxed), Ordering::Relaxed);
            self.playing.store(false, Ordering::Relaxed);
        } else {
            self.position
                .store(position.max(0.0) as u64, Ordering::Relaxed);
        }
    }
}

fn soft_clip(value: f32) -> f32 {
    value.clamp(-1.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::decode::{SideAudio, TrackMarker};
    use std::f32::consts::TAU;

    fn side(seconds: f64, frequency: f32, amplitude: f32) -> Arc<SideAudio> {
        let rate = 48_000u32;
        let frames = (seconds * f64::from(rate)) as usize;
        let mut samples = Vec::with_capacity(frames * 2);
        for frame in 0..frames {
            let value = (TAU * frequency * frame as f32 / rate as f32).sin() * amplitude;
            samples.push(value);
            samples.push(value);
        }
        Arc::new(SideAudio {
            samples,
            rate,
            duration: seconds,
            tracks: vec![TrackMarker {
                index: 0,
                start: 0,
                end: frames as u64,
            }],
        })
    }

    fn render(engine: &Engine, dsp: &mut DspState, frames: usize) -> Vec<f32> {
        let mut out = vec![0.0; frames * 2];
        engine.process(&mut out, 48_000, dsp);
        out
    }

    #[test]
    fn advances_position_at_unit_rate() {
        let engine = Engine::new();
        engine.set_side(Some(side(1.0, 440.0, 0.5)));
        engine.play();
        let mut dsp = DspState::new();
        render(&engine, &mut dsp, 4_800);
        assert_eq!(engine.position_frames(), 4_800);
    }

    #[test]
    fn renders_audio_and_stops_at_the_end() {
        let engine = Engine::new();
        engine.set_side(Some(side(0.5, 440.0, 0.5)));
        engine.play();
        let mut dsp = DspState::new();
        let out = render(&engine, &mut dsp, 24_000);
        let peak = out.iter().fold(0.0f32, |acc, value| acc.max(value.abs()));
        assert!(peak > 0.2, "peak {peak}");
        assert!(!engine.is_playing(), "must stop at the end");
        assert!((engine.position_seconds() - 0.5).abs() < 1e-3);
        let tail = render(&engine, &mut dsp, 480);
        assert!(tail.iter().all(|value| *value == 0.0));
    }

    #[test]
    fn seeks_are_sample_accurate() {
        let engine = Engine::new();
        engine.set_side(Some(side(2.0, 440.0, 0.5)));
        engine.seek_seconds(1.5);
        assert_eq!(engine.position_frames(), 72_000);
        engine.play();
        let mut dsp = DspState::new();
        render(&engine, &mut dsp, 480);
        assert_eq!(engine.position_frames(), 72_480);
    }

    #[test]
    fn honors_varispeed() {
        let engine = Engine::new();
        engine.set_side(Some(side(2.0, 440.0, 0.5)));
        engine.set_rate(1.35);
        engine.play();
        let mut dsp = DspState::new();
        render(&engine, &mut dsp, 4_800);
        assert!((engine.position_frames() as i64 - 6_480).abs() <= 1);
    }

    #[test]
    fn rate_changes_do_not_click() {
        let engine = Engine::new();
        engine.set_side(Some(side(2.0, 440.0, 0.5)));
        engine.play();
        let mut dsp = DspState::new();
        let mut out = render(&engine, &mut dsp, 1_000);
        engine.set_rate(1.35);
        out.extend(render(&engine, &mut dsp, 1_000));

        let max_delta = out
            .windows(2)
            .map(|pair| (pair[1] - pair[0]).abs())
            .fold(0.0f32, f32::max);
        assert!(max_delta < 0.1, "click: {max_delta}");
    }

    #[test]
    fn silence_plus_noise_only_when_character_is_on() {
        let engine = Engine::new();
        engine.set_side(Some(side(1.0, 0.0, 0.0)));
        engine.set_character(0.0);
        engine.play();
        let mut dsp = DspState::new();
        let out = render(&engine, &mut dsp, 4_800);
        assert!(out.iter().all(|value| *value == 0.0));

        let engine = Engine::new();
        engine.set_side(Some(side(1.0, 0.0, 0.0)));
        engine.set_character(1.0);
        engine.play();
        let mut dsp = DspState::new();
        let out = render(&engine, &mut dsp, 48_000);
        let energy: f32 = out.iter().map(|value| value.abs()).sum();
        assert!(energy > 0.01, "vinyl noise missing: {energy}");
    }

    #[test]
    fn limiter_keeps_output_in_range() {
        let engine = Engine::new();
        engine.set_side(Some(side(1.0, 220.0, 3.0)));
        engine.set_character(0.0);
        engine.set_volume(1.5);
        engine.play();
        let mut dsp = DspState::new();
        let out = render(&engine, &mut dsp, 4_800);
        assert!(out.iter().all(|value| value.abs() <= 1.0));
    }

    #[test]
    fn play_after_the_end_restarts() {
        let engine = Engine::new();
        engine.set_side(Some(side(0.2, 440.0, 0.5)));
        engine.play();
        let mut dsp = DspState::new();
        render(&engine, &mut dsp, 12_000);
        assert!(!engine.is_playing());
        assert!(engine.position_frames() > 0);

        engine.play();
        assert!(engine.is_playing());
        assert_eq!(engine.position_frames(), 0);
    }

    #[test]
    fn stop_resets_position() {
        let engine = Engine::new();
        engine.set_side(Some(side(1.0, 440.0, 0.5)));
        engine.play();
        let mut dsp = DspState::new();
        render(&engine, &mut dsp, 1_000);
        engine.stop();
        assert_eq!(engine.position_frames(), 0);
        assert!(!engine.is_playing());
    }
}
