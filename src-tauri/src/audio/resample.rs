//! Interpolation and sample-rate conversion.
//!
//! Playback uses a Catmull-Rom interpolator at a fractional source position so
//! varispeed and (later) reverse scratching stay sample accurate.

/// Four-point Catmull-Rom interpolation between `p1` and `p2`.
pub fn catmull_rom(p0: f32, p1: f32, p2: f32, p3: f32, t: f32) -> f32 {
    let t2 = t * t;
    let t3 = t2 * t;
    0.5 * ((2.0 * p1)
        + (-p0 + p2) * t
        + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2
        + (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * t3)
}

fn frame_sample(
    samples: &[f32],
    frames: usize,
    channels: usize,
    frame: isize,
    channel: usize,
) -> f32 {
    let index = frame.clamp(0, frames.saturating_sub(1) as isize) as usize;
    samples[index * channels + channel]
}

/// Reads one interpolated frame at fractional `position` (in frames).
pub fn read_interleaved(
    samples: &[f32],
    frames: usize,
    channels: usize,
    position: f64,
    out: &mut [f32],
) {
    if frames == 0 || channels == 0 {
        out.fill(0.0);
        return;
    }

    let base = position.floor();
    let frac = (position - base) as f32;
    let base = base as isize;

    for channel in 0..out.len().min(channels) {
        let p0 = frame_sample(samples, frames, channels, base - 1, channel);
        let p1 = frame_sample(samples, frames, channels, base, channel);
        let p2 = frame_sample(samples, frames, channels, base + 1, channel);
        let p3 = frame_sample(samples, frames, channels, base + 2, channel);
        out[channel] = catmull_rom(p0, p1, p2, p3, frac);
    }
}

/// Converts interleaved audio to another sample rate (offline, used by the
/// loader).
pub fn resample_to(samples: &[f32], src_rate: u32, channels: usize, dst_rate: u32) -> Vec<f32> {
    if channels == 0 || samples.is_empty() {
        return Vec::new();
    }
    if src_rate == dst_rate || src_rate == 0 || dst_rate == 0 {
        return samples.to_vec();
    }

    let frames = samples.len() / channels;
    let out_frames = ((frames as f64) * f64::from(dst_rate) / f64::from(src_rate)).round() as usize;
    let step = f64::from(src_rate) / f64::from(dst_rate);
    let mut out = vec![0.0; out_frames * channels];
    let mut frame = [0.0; 8];

    for index in 0..out_frames {
        let channels_here = channels.min(frame.len());
        read_interleaved(
            samples,
            frames,
            channels,
            index as f64 * step,
            &mut frame[..channels_here],
        );
        out[index * channels..index * channels + channels_here]
            .copy_from_slice(&frame[..channels_here]);
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::TAU;

    #[test]
    fn interpolates_endpoints_exactly() {
        assert!((catmull_rom(0.0, 1.0, 2.0, 3.0, 0.0) - 1.0).abs() < 1e-6);
        assert!((catmull_rom(0.0, 1.0, 2.0, 3.0, 1.0) - 2.0).abs() < 1e-6);
    }

    #[test]
    fn reproduces_a_linear_ramp() {
        for step in 0..10 {
            let t = step as f32 / 10.0;
            let value = catmull_rom(0.0, 1.0, 2.0, 3.0, t);
            assert!((value - (1.0 + t)).abs() < 1e-5);
        }
    }

    #[test]
    fn resamples_a_sine_to_48k() {
        let src_rate = 44_100;
        let seconds = 0.5;
        let freq = 440.0;
        let frames = (f64::from(src_rate) * seconds) as usize;
        let mut samples = Vec::with_capacity(frames * 2);
        for frame in 0..frames {
            let value = (TAU * freq * frame as f32 / src_rate as f32).sin() * 0.5;
            samples.push(value);
            samples.push(value);
        }

        let out = resample_to(&samples, src_rate, 2, 48_000);
        let out_frames = out.len() / 2;
        let expected = (f64::from(48_000) * seconds) as usize;
        assert!(
            out_frames.abs_diff(expected) <= 2,
            "{out_frames} vs {expected}"
        );

        let mut crossings = 0;
        for frame in 1..out_frames {
            if out[frame * 2 - 2] <= 0.0 && out[frame * 2] > 0.0 {
                crossings += 1;
            }
        }
        let measured = crossings as f64 / seconds;
        assert!(
            (measured - f64::from(freq)).abs() < 10.0,
            "measured {measured} Hz"
        );
    }

    #[test]
    fn keeps_stereo_channels_apart() {
        let samples = [1.0, -1.0, 1.0, -1.0];
        let out = resample_to(&samples, 48_000, 2, 48_000);
        assert_eq!(out, samples);
    }

    #[test]
    fn handles_empty_input() {
        assert!(resample_to(&[], 44_100, 2, 48_000).is_empty());
    }
}
