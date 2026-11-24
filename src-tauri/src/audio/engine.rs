//! Playback engine: turns the loaded track into stereo output.
//!
//! The engine holds the current track and an optional prefetched next track. At
//! the end of a track it switches to the next buffer without going through the
//! loader, so an album plays across track boundaries. Only the callback-owned
//! [`DspState`], atomics and an `ArcSwapOption` are touched; retired buffers are
//! handed back to the session for deallocation off the audio thread.

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use arc_swap::ArcSwapOption;

use super::cue::{CueVoice, RunoutVoice};
use super::decode::TrackAudio;
use super::noise::VinylNoise;
use super::resample::read_interleaved;

const FADE_SECONDS: f64 = 0.02;
/// Default time the stylus rides the runout groove before the arm returns.
const RUNOUT_SECONDS: f32 = 5.0;

pub struct DspState {
    noise: VinylNoise,
    cue: CueVoice,
    lowpass: [f32; 2],
    time: f64,
    was_playing: bool,
    pre_roll: u64,
    runout: u64,
    runout_started: bool,
    runout_voice: RunoutVoice,
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
            cue: CueVoice::new(),
            lowpass: [0.0; 2],
            time: 0.0,
            was_playing: false,
            pre_roll: 0,
            runout: 0,
            runout_started: false,
            runout_voice: RunoutVoice::new(),
        }
    }
}

pub struct Engine {
    current: ArcSwapOption<TrackAudio>,
    next: ArcSwapOption<TrackAudio>,
    playing: AtomicBool,
    position: AtomicU64,
    rate: AtomicU32,
    volume: AtomicU32,
    character: AtomicU32,
    needle_pre_roll: AtomicBool,
    suppress_pre_roll: AtomicBool,
    cue_drop: AtomicBool,
    runout_seconds: AtomicU32,
    track_index: AtomicU64,
    side_start: AtomicU64,
    retired: Mutex<Vec<Arc<TrackAudio>>>,
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
            current: ArcSwapOption::empty(),
            next: ArcSwapOption::empty(),
            playing: AtomicBool::new(false),
            position: AtomicU64::new(0),
            rate: AtomicU32::new(1.0f32.to_bits()),
            volume: AtomicU32::new(0.9f32.to_bits()),
            character: AtomicU32::new(0.3f32.to_bits()),
            needle_pre_roll: AtomicBool::new(false),
            suppress_pre_roll: AtomicBool::new(false),
            cue_drop: AtomicBool::new(false),
            runout_seconds: AtomicU32::new(RUNOUT_SECONDS.to_bits()),
            track_index: AtomicU64::new(0),
            side_start: AtomicU64::new(0.0f64.to_bits()),
            retired: Mutex::new(Vec::with_capacity(4)),
        }
    }

    /// Replaces the current track and seeks to `start_seconds` inside its buffer.
    pub fn load_track(&self, track: Arc<TrackAudio>, start_seconds: f64) {
        let local = (start_seconds.max(0.0) * f64::from(track.rate)) as u64;
        self.track_index
            .store(track.track_index as u64, Ordering::Relaxed);
        self.side_start
            .store(track.side_start.to_bits(), Ordering::Relaxed);
        self.position.store(local, Ordering::Relaxed);
        self.current.store(Some(track));
    }

    pub fn clear(&self) {
        self.playing.store(false, Ordering::Relaxed);
        self.position.store(0, Ordering::Relaxed);
        self.current.store(None);
        self.next.store(None);
    }

    pub fn queue_next(&self, track: Arc<TrackAudio>) {
        self.next.store(Some(track));
    }

    pub fn clear_next(&self) {
        self.next.store(None);
    }

    pub fn has_next(&self) -> bool {
        self.next.load().is_some()
    }

    /// Takes the prefetched track if it is the requested one.
    pub fn take_next(&self, index: usize) -> Option<Arc<TrackAudio>> {
        let next = self.next.load_full()?;
        if next.track_index != index {
            return None;
        }
        self.next.store(None);
        Some(next)
    }

    pub fn has_track(&self) -> bool {
        self.current.load().is_some()
    }

    pub fn play(&self) {
        let Some(track) = self.current.load_full() else {
            return;
        };
        let frames = (track.duration * f64::from(track.rate)) as u64;
        if self.position.load(Ordering::Relaxed) >= frames {
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

    pub fn track_index(&self) -> usize {
        self.track_index.load(Ordering::Relaxed) as usize
    }

    pub fn position_frames(&self) -> u64 {
        self.position.load(Ordering::Relaxed)
    }

    pub fn position_seconds(&self) -> f64 {
        let rate = self
            .current
            .load()
            .as_ref()
            .map(|track| track.rate)
            .unwrap_or(48_000);
        self.position_frames() as f64 / f64::from(rate)
    }

    /// Position within the whole side, including the track offset.
    pub fn side_position_seconds(&self) -> f64 {
        f64::from_bits(self.side_start.load(Ordering::Relaxed)) + self.position_seconds()
    }

    pub fn duration_seconds(&self) -> f64 {
        self.current
            .load()
            .as_ref()
            .map(|track| track.duration)
            .unwrap_or(0.0)
    }

    pub fn seek_local_seconds(&self, seconds: f64) {
        let Some(track) = self.current.load_full() else {
            return;
        };
        let frames = (track.duration * f64::from(track.rate)) as u64;
        let local = (seconds.max(0.0) * f64::from(track.rate)) as u64;
        self.position.store(local.min(frames), Ordering::Relaxed);
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

    /// The session enables this so the needle lands before the first note.
    pub fn set_needle_pre_roll(&self, enabled: bool) {
        self.needle_pre_roll.store(enabled, Ordering::Relaxed);
    }

    /// Manual cueing places the needle by hand: play the drop, but not the
    /// lead-in static.
    pub fn request_cue_drop(&self) {
        self.cue_drop.store(true, Ordering::Relaxed);
    }

    /// Suppresses the next lead-in completely (a plain seek, no drop).
    pub fn suppress_next_pre_roll(&self) {
        self.suppress_pre_roll.store(true, Ordering::Relaxed);
    }

    /// How long the stylus rides the runout groove at the end of a side.
    pub fn set_runout_seconds(&self, seconds: f64) {
        store_f32(&self.runout_seconds, seconds.max(0.0) as f32);
    }

    pub fn character(&self) -> f32 {
        load_f32(&self.character)
    }

    /// Drops buffers retired by the audio thread. Call from the session thread.
    pub fn drain_retired(&self) {
        if let Ok(mut retired) = self.retired.lock() {
            retired.clear();
        }
    }

    fn retire(&self, old: Option<Arc<TrackAudio>>) {
        let Some(old) = old else {
            return;
        };
        if let Ok(mut retired) = self.retired.try_lock()
            && retired.len() < 4
        {
            retired.push(old);
        }
    }

    fn advance(&self) -> bool {
        let Some(next) = self.next.swap(None) else {
            return false;
        };
        let old = self.current.swap(Some(next.clone()));
        self.position.store(0, Ordering::Relaxed);
        self.track_index
            .store(next.track_index as u64, Ordering::Relaxed);
        self.side_start
            .store(next.side_start.to_bits(), Ordering::Relaxed);
        self.retire(old);
        true
    }

    /// Renders one device buffer with an arbitrary channel count.
    ///
    /// The mix is stereo; on a multi-output interface the first two channels
    /// carry left/right and the remaining outputs stay silent. Mono devices get
    /// a downmix. `scratch` is reused between calls so the callback does not
    /// allocate.
    pub fn process_device(
        &self,
        out: &mut [f32],
        channels: usize,
        device_rate: u32,
        dsp: &mut DspState,
        scratch: &mut Vec<f32>,
    ) {
        if channels == 0 {
            out.fill(0.0);
            return;
        }

        let frames = out.len() / channels;
        if frames == 0 {
            return;
        }

        scratch.resize(frames * 2, 0.0);
        self.process(&mut scratch[..frames * 2], device_rate, dsp);

        if channels == 1 {
            for frame in 0..frames {
                out[frame] = (scratch[frame * 2] + scratch[frame * 2 + 1]) * 0.5;
            }
        } else {
            for frame in 0..frames {
                out[frame * channels] = scratch[frame * 2];
                out[frame * channels + 1] = scratch[frame * 2 + 1];
                for channel in 2..channels {
                    out[frame * channels + channel] = 0.0;
                }
            }
        }

        let used = frames * channels;
        if used < out.len() {
            out[used..].fill(0.0);
        }
    }

    /// Renders interleaved stereo f32.
    pub fn process(&self, out: &mut [f32], device_rate: u32, dsp: &mut DspState) {
        out.fill(0.0);
        if out.len() < 2 {
            return;
        }

        if !self.playing.load(Ordering::Relaxed) {
            dsp.was_playing = false;
            dsp.runout = 0;
            dsp.runout_started = false;
            return;
        }

        let mut current = self.current.load_full();
        if current.is_none() {
            dsp.was_playing = false;
            self.playing.store(false, Ordering::Relaxed);
            return;
        }

        // Consume queued start hints even if playback is already running, so
        // they cannot leak into the next real start.
        let suppressed = self.suppress_pre_roll.swap(false, Ordering::Relaxed);
        let cue_drop = self.cue_drop.swap(false, Ordering::Relaxed);

        let device_rate = device_rate.max(1);
        let base_rate = f64::from(load_f32(&self.rate))
            * (f64::from(
                current
                    .as_ref()
                    .map(|track| track.rate)
                    .unwrap_or(device_rate),
            ) / f64::from(device_rate));
        let character = load_f32(&self.character);
        let volume = load_f32(&self.volume);
        let sample_rate = f64::from(device_rate);

        if !dsp.was_playing {
            // The lead-in belongs to the start of the side, not to the start of
            // every track: a cue into track 3 must not replay the static.
            let at_side_start = self.side_position_seconds() <= 0.05;
            if cue_drop {
                // Placing the needle by hand: the drop plays, then the music.
                let seconds = dsp.cue.start_drop(device_rate);
                dsp.pre_roll = (f64::from(seconds) * f64::from(device_rate)) as u64;
            } else if at_side_start && self.needle_pre_roll.load(Ordering::Relaxed) && !suppressed {
                // Let the needle land and the lead-in groove run before the
                // first note. The track position stays put during this pre-roll.
                let seconds = dsp.cue.start(device_rate);
                dsp.pre_roll = (f64::from(seconds) * f64::from(device_rate)) as u64;
            } else {
                dsp.pre_roll = 0;
            }
            dsp.was_playing = true;
        }

        let mut frame_out = [0.0f32; 2];
        let mut position = self.position.load(Ordering::Relaxed) as f64;
        let mut finished = false;

        for frame in out.as_chunks_mut::<2>().0 {
            if dsp.runout > 0 {
                // The side is over, but the stylus rides the locked groove for
                // a few more seconds before the arm returns.
                let (left, right) = dsp.runout_voice.process(device_rate);
                let fade = (dsp.runout as f32 / (0.4 * device_rate as f32)).min(1.0);
                frame[0] = soft_clip(left * volume * fade);
                frame[1] = soft_clip(right * volume * fade);
                dsp.runout -= 1;
                if dsp.runout == 0 {
                    finished = true;
                }
                continue;
            }

            // Switch to the prefetched track at the boundary.
            let mut side_ended = false;
            loop {
                let Some(track) = current.as_ref() else {
                    side_ended = true;
                    break;
                };
                let frames = track.samples.len() / 2;
                if position < (frames.saturating_sub(2)) as f64 {
                    break;
                }
                if !self.advance() {
                    side_ended = true;
                    break;
                }
                current = self.current.load_full();
                position = 0.0;
            }
            if side_ended {
                if !dsp.runout_started {
                    dsp.runout_started = true;
                    dsp.runout =
                        (f64::from(load_f32(&self.runout_seconds)) * f64::from(device_rate))
                            as u64;
                    dsp.runout_voice.start(device_rate);
                    continue;
                }
                finished = true;
                break;
            }

            if dsp.pre_roll > 0 {
                // Real needle drop and lead-in surface noise. Fade the last
                // 30 ms so the start of the music does not click.
                let (cue_left, cue_right) = dsp.cue.process(device_rate);
                let fade = (dsp.pre_roll as f32 / (0.03 * device_rate as f32)).min(1.0);
                frame[0] = soft_clip(cue_left * volume * fade);
                frame[1] = soft_clip(cue_right * volume * fade);
                dsp.pre_roll -= 1;
                dsp.time += 1.0 / sample_rate;
                continue;
            }

            let Some(track) = current.as_ref() else {
                finished = true;
                break;
            };
            let frames = track.samples.len() / 2;
            if frames < 4 {
                finished = true;
                break;
            }

            let wow = 1.0
                + f64::from(character)
                    * (0.0015 * (std::f64::consts::TAU * 0.55 * dsp.time).sin()
                        + 0.0005 * (std::f64::consts::TAU * 6.3 * dsp.time).sin());
            let rate = base_rate * wow;

            read_interleaved(&track.samples, frames, 2, position, &mut frame_out);

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

            // A short fade keeps gapless track changes free of clicks.
            let fade = (FADE_SECONDS * f64::from(track.rate)).max(1.0);
            let gain = ((position / fade).min(1.0) * ((frames as f64 - position) / fade).min(1.0))
                .clamp(0.0, 1.0) as f32;
            left *= gain;
            right *= gain;

            let (noise_left, noise_right) = dsp.noise.process(character, device_rate, 1.0);
            frame[0] = soft_clip((left + noise_left) * volume);
            frame[1] = soft_clip((right + noise_right) * volume);

            position += rate;
            dsp.time += 1.0 / sample_rate;
        }

        if finished {
            let end = current
                .as_ref()
                .map(|track| track.samples.len() / 2)
                .unwrap_or(position as usize);
            self.position.store(end as u64, Ordering::Relaxed);
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
    use crate::audio::decode::TrackAudio;
    use std::f32::consts::TAU;

    fn track(seconds: f64, frequency: f32, amplitude: f32) -> Arc<TrackAudio> {
        track_at(0, 0.0, seconds, frequency, amplitude)
    }

    fn track_at(
        index: usize,
        side_start: f64,
        seconds: f64,
        frequency: f32,
        amplitude: f32,
    ) -> Arc<TrackAudio> {
        let rate = 48_000u32;
        let frames = (seconds * f64::from(rate)) as usize;
        let mut samples = Vec::with_capacity(frames * 2);
        for frame in 0..frames {
            let value = (TAU * frequency * frame as f32 / rate as f32).sin() * amplitude;
            samples.push(value);
            samples.push(value);
        }
        Arc::new(TrackAudio {
            samples,
            rate,
            duration: seconds,
            pregap: 0.0,
            track_index: index,
            side_start,
        })
    }

    fn render(engine: &Engine, dsp: &mut DspState, frames: usize) -> Vec<f32> {
        let mut out = vec![0.0; frames * 2];
        engine.process(&mut out, 48_000, dsp);
        out
    }

    #[test]
    fn start_of_side_plays_the_lead_in() {
        let engine = Engine::new();
        engine.set_needle_pre_roll(true);
        engine.load_track(track(1.0, 440.0, 0.5), 0.0);
        engine.play();
        let mut dsp = DspState::new();
        render(&engine, &mut dsp, 480);
        assert!(dsp.pre_roll > 0, "the side start should play the lead-in");
    }

    #[test]
    fn cueing_mid_side_skips_the_lead_in() {
        let engine = Engine::new();
        engine.set_needle_pre_roll(true);
        engine.load_track(track_at(2, 61.5, 1.0, 440.0, 0.5), 0.0);
        engine.play();
        let mut dsp = DspState::new();
        render(&engine, &mut dsp, 480);
        assert_eq!(dsp.pre_roll, 0, "no lead-in when starting mid-side");
    }

    #[test]
    fn a_manual_cue_plays_only_the_drop() {
        let engine = Engine::new();
        engine.set_needle_pre_roll(true);
        engine.load_track(track(1.0, 440.0, 0.5), 0.0);
        engine.request_cue_drop();
        engine.play();
        let mut dsp = DspState::new();
        render(&engine, &mut dsp, 480);
        assert!(dsp.pre_roll > 0, "a manual cue plays the drop");
        assert!(
            dsp.pre_roll <= 48_000 / 2,
            "but not the multi-second lead-in static"
        );
    }

    #[test]
    fn a_plain_seek_does_not_play_a_drop() {
        let engine = Engine::new();
        engine.set_needle_pre_roll(true);
        engine.load_track(track(1.0, 440.0, 0.5), 0.0);
        engine.suppress_next_pre_roll();
        engine.play();
        let mut dsp = DspState::new();
        render(&engine, &mut dsp, 480);
        assert_eq!(dsp.pre_roll, 0);
    }

    #[test]
    fn advances_position_at_unit_rate() {
        let engine = Engine::new();
        engine.load_track(track(1.0, 440.0, 0.5), 0.0);
        engine.play();
        let mut dsp = DspState::new();
        render(&engine, &mut dsp, 4_800);
        assert_eq!(engine.position_frames(), 4_800);
    }

    #[test]
    fn renders_audio_and_rides_the_runout_before_stopping() {
        let engine = Engine::new();
        engine.load_track(track(0.5, 440.0, 0.5), 0.0);
        engine.play();
        let mut dsp = DspState::new();
        let out = render(&engine, &mut dsp, 24_000);
        let peak = out.iter().fold(0.0f32, |acc, value| acc.max(value.abs()));
        assert!(peak > 0.2, "peak {peak}");
        assert!(engine.is_playing(), "the runout keeps the motor on");

        // The runout groove must carry sound, not silence.
        let runout = render(&engine, &mut dsp, 4_800);
        assert!(
            runout.iter().any(|value| value.abs() > 0.0),
            "the runout should be audible"
        );

        // After about five seconds the side stops and the arm can return.
        render(&engine, &mut dsp, 6 * 48_000);
        assert!(!engine.is_playing(), "must stop after the runout");
        assert!((engine.position_seconds() - 0.5).abs() < 1e-3);
        let tail = render(&engine, &mut dsp, 480);
        assert!(tail.iter().all(|value| *value == 0.0));
    }

    #[test]
    fn seeks_are_sample_accurate() {
        let engine = Engine::new();
        engine.load_track(track(2.0, 440.0, 0.5), 0.0);
        engine.seek_local_seconds(1.5);
        assert_eq!(engine.position_frames(), 72_000);
        engine.play();
        let mut dsp = DspState::new();
        render(&engine, &mut dsp, 480);
        assert_eq!(engine.position_frames(), 72_480);
    }

    #[test]
    fn side_position_includes_the_track_offset() {
        let engine = Engine::new();
        engine.load_track(track_at(2, 61.5, 2.0, 440.0, 0.5), 0.0);
        engine.seek_local_seconds(0.5);
        assert!((engine.side_position_seconds() - 62.0).abs() < 1e-6);
    }

    #[test]
    fn honors_varispeed() {
        let engine = Engine::new();
        engine.load_track(track(2.0, 440.0, 0.5), 0.0);
        engine.set_rate(1.35);
        engine.play();
        let mut dsp = DspState::new();
        render(&engine, &mut dsp, 4_800);
        assert!((engine.position_frames() as i64 - 6_480).abs() <= 1);
    }

    #[test]
    fn rate_changes_do_not_click() {
        let engine = Engine::new();
        engine.load_track(track(2.0, 440.0, 0.5), 0.0);
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
    fn auto_advances_to_the_queued_track() {
        let engine = Engine::new();
        engine.load_track(track_at(0, 0.0, 0.5, 440.0, 0.5), 0.0);
        engine.queue_next(track_at(1, 0.5, 0.5, 660.0, 0.5));
        engine.play();
        let mut dsp = DspState::new();
        render(&engine, &mut dsp, 36_000);
        assert_eq!(engine.track_index(), 1);
        assert!(engine.is_playing());
        assert!((engine.side_position_seconds() - 0.75).abs() < 1e-3);
        assert!(!engine.has_next());
    }

    #[test]
    fn track_change_is_faded() {
        let engine = Engine::new();
        engine.load_track(track_at(0, 0.0, 0.3, 440.0, 0.9), 0.0);
        engine.queue_next(track_at(1, 0.3, 0.3, 660.0, 0.9));
        engine.play();
        let mut dsp = DspState::new();
        let out = render(&engine, &mut dsp, 28_000);
        let max_delta = out
            .windows(2)
            .map(|pair| (pair[1] - pair[0]).abs())
            .fold(0.0f32, f32::max);
        assert!(max_delta < 0.25, "click at track change: {max_delta}");
    }

    #[test]
    fn silence_plus_noise_only_when_character_is_on() {
        let engine = Engine::new();
        engine.load_track(track(1.0, 0.0, 0.0), 0.0);
        engine.set_character(0.0);
        engine.play();
        let mut dsp = DspState::new();
        let out = render(&engine, &mut dsp, 4_800);
        assert!(out.iter().all(|value| *value == 0.0));

        let engine = Engine::new();
        engine.load_track(track(1.0, 0.0, 0.0), 0.0);
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
        engine.load_track(track(1.0, 220.0, 3.0), 0.0);
        engine.set_character(0.0);
        engine.set_volume(1.5);
        engine.play();
        let mut dsp = DspState::new();
        let out = render(&engine, &mut dsp, 4_800);
        assert!(out.iter().all(|value| value.abs() <= 1.0));
    }

    #[test]
    fn maps_stereo_onto_a_multichannel_interface() {
        let engine = Engine::new();
        engine.load_track(track(1.0, 440.0, 0.5), 0.0);
        engine.set_character(0.0);
        engine.play();
        let mut dsp = DspState::new();
        let mut scratch = Vec::new();
        let mut out = vec![0.0; 480 * 6];
        engine.process_device(&mut out, 6, 48_000, &mut dsp, &mut scratch);

        assert!(
            (0..480).any(|frame| out[frame * 6].abs() > 0.0),
            "left channel missing"
        );
        for frame in 0..480 {
            for channel in 2..6 {
                assert_eq!(out[frame * 6 + channel], 0.0, "unused output not silent");
            }
        }

        let reference_engine = Engine::new();
        reference_engine.load_track(track(1.0, 440.0, 0.5), 0.0);
        reference_engine.set_character(0.0);
        reference_engine.play();
        let mut reference_dsp = DspState::new();
        let mut stereo = vec![0.0; 480 * 2];
        reference_engine.process(&mut stereo, 48_000, &mut reference_dsp);

        for frame in 0..480 {
            assert!((out[frame * 6] - stereo[frame * 2]).abs() < 1e-6);
            assert!((out[frame * 6 + 1] - stereo[frame * 2 + 1]).abs() < 1e-6);
        }
    }

    #[test]
    fn downmixes_for_mono_devices() {
        let engine = Engine::new();
        engine.load_track(track(1.0, 440.0, 0.4), 0.0);
        engine.set_character(0.0);
        engine.play();
        let mut dsp = DspState::new();
        let mut scratch = Vec::new();
        let mut out = vec![0.0; 480];
        engine.process_device(&mut out, 1, 48_000, &mut dsp, &mut scratch);
        assert!(out.iter().any(|value| value.abs() > 0.0));
        assert!(out.iter().all(|value| value.abs() <= 0.4 + 1e-6));
    }

    #[test]
    fn ignores_an_incomplete_frame() {
        let engine = Engine::new();
        engine.load_track(track(1.0, 440.0, 0.5), 0.0);
        engine.play();
        let mut dsp = DspState::new();
        let mut scratch = Vec::new();
        let mut out = vec![1.0; 480 * 2 + 1];
        engine.process_device(&mut out, 2, 48_000, &mut dsp, &mut scratch);
        assert_eq!(out[out.len() - 1], 0.0);
    }

    #[test]
    fn takes_only_the_matching_prefetched_track() {
        let engine = Engine::new();
        engine.load_track(track_at(0, 0.0, 0.2, 440.0, 0.5), 0.0);
        engine.queue_next(track_at(1, 0.2, 0.2, 660.0, 0.5));
        assert!(engine.take_next(2).is_none());
        assert!(engine.has_next());
        let taken = engine.take_next(1).expect("prefetched track");
        assert_eq!(taken.track_index, 1);
        assert!(!engine.has_next());
        assert!(engine.take_next(1).is_none());
    }

    #[test]
    fn play_after_the_end_restarts() {
        let engine = Engine::new();
        engine.load_track(track(0.2, 440.0, 0.5), 0.0);
        engine.play();
        let mut dsp = DspState::new();
        render(&engine, &mut dsp, 12_000);
        render(&engine, &mut dsp, 6 * 48_000);
        assert!(!engine.is_playing());
        assert!(engine.position_frames() > 0);

        engine.play();
        assert!(engine.is_playing());
        assert_eq!(engine.position_frames(), 0);
    }

    #[test]
    fn stop_resets_position() {
        let engine = Engine::new();
        engine.load_track(track(1.0, 440.0, 0.5), 0.0);
        engine.play();
        let mut dsp = DspState::new();
        render(&engine, &mut dsp, 1_000);
        engine.stop();
        assert_eq!(engine.position_frames(), 0);
        assert!(!engine.is_playing());
    }

    #[test]
    fn first_play_holds_the_track_during_the_needle_pre_roll() {
        let engine = Engine::new();
        engine.set_needle_pre_roll(true);
        engine.load_track(track(1.0, 440.0, 0.5), 0.0);
        engine.play();
        let mut dsp = DspState::new();
        let mut out = vec![0.0; 4_800 * 2];
        engine.process(&mut out, 48_000, &mut dsp);
        assert_eq!(engine.position_frames(), 0, "music must wait for the needle");
        assert!(
            out.iter().any(|sample| sample.abs() > 1e-5),
            "the pre-roll must not be silent"
        );
    }

    #[test]
    fn music_starts_after_the_needle_pre_roll() {
        let engine = Engine::new();
        engine.set_needle_pre_roll(true);
        engine.load_track(track(2.0, 440.0, 0.5), 0.0);
        engine.play();
        let mut dsp = DspState::new();
        let mut out = vec![0.0; 240_000 * 2];
        engine.process(&mut out, 48_000, &mut dsp);
        assert!(engine.position_frames() > 0, "position advances after the drop");
    }

    #[test]
    fn needle_pre_roll_lasts_more_than_a_second() {
        let engine = Engine::new();
        engine.set_needle_pre_roll(true);
        engine.load_track(track(3.0, 440.0, 0.5), 0.0);
        engine.play();
        let mut dsp = DspState::new();
        let mut out = vec![0.0; 48_000 * 2];
        engine.process(&mut out, 48_000, &mut dsp);
        assert_eq!(engine.position_frames(), 0, "the pre-roll should still hold");
        let energy: f32 = out.iter().map(|sample| sample.abs()).sum();
        assert!(energy > 50.0, "the pre-roll should be clearly audible: {energy}");
        let peak = out.iter().fold(0.0f32, |peak, sample| peak.max(sample.abs()));
        assert!(peak > 0.15, "needle drop should have a clear peak: {peak}");
    }

    #[test]
    fn resume_mid_side_does_not_wait_for_the_needle() {
        let engine = Engine::new();
        engine.load_track(track(2.0, 440.0, 0.5), 0.5);
        engine.play();
        let mut dsp = DspState::new();
        let mut out = vec![0.0; 4_800 * 2];
        engine.process(&mut out, 48_000, &mut dsp);
        assert!(engine.position_seconds() > 0.55);
    }
}
