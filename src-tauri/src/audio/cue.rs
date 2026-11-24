//! Real recordings for the needle touchdown and the lead-in surface noise.
//!
//! The two mono WAV files are bundled with the app. Every time the needle is
//! cued we pick a new random start inside the static recording, a slightly
//! different playback rate and a different gain, so repeated starts never sound
//! identical.

use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

/// A decoded mono sample, normalized to `-1.0..=1.0`.
struct Sample {
    data: Vec<f32>,
    rate: u32,
}

impl Sample {
    fn empty() -> Self {
        Self {
            data: Vec::new(),
            rate: 48_000,
        }
    }
}

struct CueSamples {
    needle: Sample,
    surface: Sample,
}

static SAMPLES: OnceLock<CueSamples> = OnceLock::new();

fn samples() -> &'static CueSamples {
    SAMPLES.get_or_init(|| CueSamples {
        needle: parse_wav(include_bytes!("../../assets/audio/needle_drop.wav"))
            .unwrap_or_else(Sample::empty),
        surface: parse_wav(include_bytes!("../../assets/audio/vinyl_static.wav"))
            .unwrap_or_else(Sample::empty),
    })
}

/// Minimal RIFF reader for 16-bit PCM WAV files.
fn parse_wav(bytes: &[u8]) -> Option<Sample> {
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return None;
    }

    let mut position = 12usize;
    let mut channels = 1u16;
    let mut rate = 48_000u32;
    let mut bits = 16u16;
    let mut format = 1u16;
    let mut data: Option<&[u8]> = None;

    while position + 8 <= bytes.len() {
        let id = &bytes[position..position + 4];
        let length = u32::from_le_bytes(bytes[position + 4..position + 8].try_into().ok()?) as usize;
        position += 8;
        let end = (position + length).min(bytes.len());
        let chunk = &bytes[position..end];

        if id == b"fmt " && chunk.len() >= 16 {
            format = u16::from_le_bytes(chunk[0..2].try_into().ok()?);
            channels = u16::from_le_bytes(chunk[2..4].try_into().ok()?);
            rate = u32::from_le_bytes(chunk[4..8].try_into().ok()?);
            bits = u16::from_le_bytes(chunk[14..16].try_into().ok()?);
        } else if id == b"data" {
            data = Some(chunk);
        }

        position = end + (length & 1);
    }

    if format != 1 || bits != 16 || channels == 0 {
        return None;
    }
    let data = data?;
    let frame_bytes = 2 * usize::from(channels);
    let mut out = Vec::with_capacity(data.len() / frame_bytes);
    for frame in data.chunks_exact(frame_bytes) {
        let mut sum = 0.0f32;
        for channel in 0..usize::from(channels) {
            let offset = channel * 2;
            sum += f32::from(i16::from_le_bytes([frame[offset], frame[offset + 1]]));
        }
        out.push(sum / (32_768.0 * f32::from(channels)));
    }

    Some(Sample { data: out, rate })
}

/// One cue playback: a needle drop and a window of real surface noise.
pub struct CueVoice {
    needle_pos: f32,
    needle_step: f32,
    needle_gain: f32,
    needle_presence: f32,
    needle_lp: f32,
    needle_active: bool,
    surface_pos: f32,
    surface_step: f32,
    surface_gain: f32,
    surface_left: f32,
    rng: u32,
    surface_ramp: f32,
}

impl CueVoice {
    pub fn new() -> Self {
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.subsec_nanos() ^ duration.as_secs() as u32)
            .unwrap_or(0x51f0_2c3a);
        Self {
            needle_pos: 0.0,
            needle_step: 1.0,
            needle_gain: 1.0,
            needle_presence: 0.0,
            needle_lp: 0.0,
            needle_active: false,
            surface_pos: 0.0,
            surface_step: 1.0,
            surface_gain: 1.0,
            surface_left: 0.0,
            rng: seed | 1,
            surface_ramp: 0.0,
        }
    }

    fn random(&mut self) -> f32 {
        let mut x = self.rng;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.rng = x;
        (x >> 8) as f32 / 16_777_216.0
    }

    fn between(&mut self, low: f32, high: f32) -> f32 {
        low + (high - low) * self.random()
    }

    /// Starts a new cue and returns the lead-in length in seconds.
    pub fn start(&mut self, device_rate: u32) -> f32 {
        let samples = samples();
        let device_rate = device_rate.max(1) as f32;

        self.needle_pos = 0.0;
        self.needle_step = samples.needle.rate as f32 / device_rate * self.between(0.96, 1.04);
        // The drop is almost pure low end (99 % below 200 Hz), so small
        // speakers barely reproduce it. A little high-passed copy of the same
        // recording adds the impact click without changing the source.
        self.needle_gain = self.between(1.0, 1.12);
        self.needle_presence = self.between(1.2, 1.4);
        self.needle_lp = 0.0;
        self.needle_active = !samples.needle.data.is_empty();

        let length = samples.surface.data.len() as f32;
        self.surface_pos = if length > 0.0 {
            self.random() * (length * 0.40)
        } else {
            0.0
        };
        self.surface_step = samples.surface.rate as f32 / device_rate * self.between(0.98, 1.02);
        // The bundled surface recording is dense and bright; at the old gain it
        // drowned both the drop and the music. Keep it as a faint bed with
        // occasional ticks, well below the music.
        self.surface_gain = self.between(0.22, 0.34);
        self.surface_left = 2.0 + 2.0 * self.random();
        self.surface_ramp = 0.0;

        self.surface_left
    }

    /// Renders the cue. Call once per output frame while the lead-in runs.
    pub fn process(&mut self, device_rate: u32) -> (f32, f32) {
        let samples = samples();
        let device_rate = device_rate.max(1) as f32;
        if self.surface_left > 0.0 {
            self.surface_left = (self.surface_left - 1.0 / device_rate).max(0.0);
        }

        let mut value = 0.0f32;
        if self.needle_active {
            // No fade-in on the drop: its impact is the point, and it starts at
            // sample 0 of a silent recording.
            let sample = read_sample(&samples.needle, &mut self.needle_pos, self.needle_step);
            let cutoff = 2.0 * std::f32::consts::PI * 1_500.0 / device_rate;
            self.needle_lp += cutoff * (sample - self.needle_lp);
            let presence = sample - self.needle_lp;
            value += self.needle_gain * (sample + self.needle_presence * presence);
            if self.needle_pos >= samples.needle.data.len() as f32 {
                self.needle_active = false;
            }
        }

        if self.surface_left > 0.0 {
            // 300 ms swell, so the static does not sit at full level from the
            // first frame; it also hides the random offset of the window.
            self.surface_ramp = (self.surface_ramp + 1.0 / (0.3 * device_rate)).min(1.0);
            value += read_sample(&samples.surface, &mut self.surface_pos, self.surface_step)
                * self.surface_gain
                * self.surface_ramp;
        }

        (value, value * 0.97)
    }
}

impl Default for CueVoice {
    fn default() -> Self {
        Self::new()
    }
}

fn read_sample(sample: &Sample, position: &mut f32, step: f32) -> f32 {
    if sample.data.is_empty() {
        return 0.0;
    }
    let length = sample.data.len();
    let index = *position as usize;
    if index >= length {
        return 0.0;
    }
    let fraction = *position - index as f32;
    let next = if index + 1 < length {
        sample.data[index + 1]
    } else {
        0.0
    };
    *position += step;
    if *position >= length as f32 {
        *position -= length as f32;
    }
    sample.data[index] * (1.0 - fraction) + next * fraction
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_samples_are_decoded() {
        let samples = samples();
        assert!(samples.needle.data.len() > 1_000);
        assert!(samples.surface.data.len() > 100_000);
        assert!(samples.needle.data.iter().any(|value| value.abs() > 0.5));
    }

    #[test]
    fn cue_varies_between_starts() {
        let mut voice = CueVoice::new();
        let first = voice.start(48_000);
        let first_pos = voice.surface_pos;
        let second = voice.start(48_000);
        assert!((2.0..=4.0).contains(&first));
        assert!((2.0..=4.0).contains(&second));
        assert!((first - second).abs() > 1e-6 || (first_pos - voice.surface_pos).abs() > 1.0);
    }

    #[test]
    fn cue_ends_after_the_lead_in() {
        let mut voice = CueVoice::new();
        let seconds = voice.start(48_000);
        let frames = (seconds * 48_000.0) as usize + 48_000;
        let mut energy = 0.0f32;
        for _ in 0..frames {
            let (left, _) = voice.process(48_000);
            energy += left.abs();
        }
        assert!(energy > 1.0, "cue should be audible");
    }
}
