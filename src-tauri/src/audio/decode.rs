//! Decoding a side into one contiguous 48 kHz stereo buffer.
//!
//! Loading a whole side keeps playback, seeking and scratching sample accurate
//! and lock free. A 22 minute side is roughly 63 MB of f32 samples.

use std::fs::File;
use std::path::Path;

use symphonia::core::codecs::audio::AudioDecoderOptions;
use symphonia::core::errors::Error;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, TrackType};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;

use crate::library::model::Side;

use super::resample::resample_to;
use super::timeline::SideTimeline;

pub struct TrackMarker {
    pub index: usize,
    pub start: u64,
    pub end: u64,
}

pub struct SideAudio {
    pub samples: Vec<f32>,
    pub rate: u32,
    pub duration: f64,
    pub tracks: Vec<TrackMarker>,
}

#[derive(Debug)]
pub struct DecodedTrack {
    pub samples: Vec<f32>,
    pub rate: u32,
    pub channels: usize,
}

#[derive(Debug, thiserror::Error)]
pub enum DecodeError {
    #[error("Datei konnte nicht geöffnet werden: {0}")]
    Io(#[from] std::io::Error),
    #[error("Dekodierung fehlgeschlagen: {0}")]
    Symphonia(#[from] Error),
    #[error("kein Audio-Track")]
    NoTrack,
    #[error("leere Audiodatei")]
    Empty,
}

pub fn decode_file(path: &Path) -> Result<DecodedTrack, DecodeError> {
    let file = File::open(path)?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(extension) = path.extension().and_then(|value| value.to_str()) {
        hint.with_extension(extension);
    }

    let mut format = symphonia::default::get_probe().probe(
        &hint,
        mss,
        FormatOptions::default(),
        MetadataOptions::default(),
    )?;

    let (track_id, mut decoder) = {
        let track = format
            .default_track(TrackType::Audio)
            .ok_or(DecodeError::NoTrack)?;
        let params = track
            .codec_params
            .as_ref()
            .and_then(|params| params.audio())
            .ok_or(DecodeError::NoTrack)?;
        let decoder = symphonia::default::get_codecs()
            .make_audio_decoder(params, &AudioDecoderOptions::default())?;
        (track.id, decoder)
    };

    let mut samples = Vec::new();
    let mut rate = 0;
    let mut channels = 0;

    while let Some(packet) = format.next_packet()? {
        if packet.track_id != track_id {
            continue;
        }
        match decoder.decode(&packet) {
            Ok(buffer) => {
                let spec = buffer.spec();
                rate = spec.rate();
                channels = spec.channels().count();
                let count = buffer.samples_interleaved();
                let offset = samples.len();
                samples.resize(offset + count, 0.0);
                buffer.copy_to_slice_interleaved(&mut samples[offset..]);
            }
            Err(Error::DecodeError(_)) | Err(Error::IoError(_)) => continue,
            Err(error) => return Err(error.into()),
        }
    }

    if rate == 0 || channels == 0 || samples.is_empty() {
        return Err(DecodeError::Empty);
    }

    Ok(DecodedTrack {
        samples,
        rate,
        channels,
    })
}

fn to_stereo(samples: &[f32], channels: usize) -> Vec<f32> {
    match channels {
        0 => Vec::new(),
        1 => samples.iter().flat_map(|value| [*value, *value]).collect(),
        2 => samples.to_vec(),
        count => samples
            .chunks_exact(count)
            .flat_map(|frame| [frame[0], frame[1]])
            .collect(),
    }
}

/// Builds the complete audio for one side, including pregaps.
pub fn build_side(
    side: &Side,
    timeline: &SideTimeline,
    target_rate: u32,
) -> (SideAudio, Vec<String>) {
    let mut samples: Vec<f32> = Vec::new();
    let mut markers = Vec::new();
    let mut issues = Vec::new();

    for span in &timeline.tracks {
        let Some(track) = side.tracks.get(span.index) else {
            continue;
        };

        let gap_frames = (span.pregap * f64::from(target_rate)).round() as usize;
        samples.resize(samples.len() + gap_frames * 2, 0.0);
        let start = (samples.len() / 2) as u64;

        let want_frames = (span.duration * f64::from(target_rate)).round() as usize;
        let decoded = decode_file(Path::new(&track.path));

        match decoded {
            Ok(decoded) => {
                let stereo = to_stereo(&decoded.samples, decoded.channels);
                let source_frames = stereo.len() / 2;
                let skip = (track.offset * f64::from(decoded.rate)).round() as usize;
                let take = (span.duration * f64::from(decoded.rate)).round() as usize;
                let end = (skip + take).min(source_frames);
                let slice: &[f32] = if skip < end {
                    &stereo[skip * 2..end * 2]
                } else {
                    &[]
                };

                let mut converted = resample_to(slice, decoded.rate, 2, target_rate);
                converted.resize(want_frames * 2, 0.0);
                converted.truncate(want_frames * 2);
                samples.extend_from_slice(&converted);
            }
            Err(error) => {
                issues.push(format!("{}: {error}", track.path));
                samples.resize(samples.len() + want_frames * 2, 0.0);
            }
        }

        markers.push(TrackMarker {
            index: span.index,
            start,
            end: (samples.len() / 2) as u64,
        });
    }

    let duration = samples.len() as f64 / (2.0 * f64::from(target_rate));

    (
        SideAudio {
            samples,
            rate: target_rate,
            duration,
            tracks: markers,
        },
        issues,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::model::{Rpm, Track};
    use std::f32::consts::TAU;
    use std::fs;
    use std::path::PathBuf;

    fn write_wav(path: &Path, seconds: f64, rate: u32, frequency: f32) {
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
            let value = (TAU * frequency * frame as f32 / rate as f32).sin() * 0.4;
            let pcm = (value * 32_767.0) as i16;
            out.extend_from_slice(&pcm.to_le_bytes());
            out.extend_from_slice(&pcm.to_le_bytes());
        }
        fs::write(path, out).unwrap();
    }

    fn temp_dir(name: &str) -> PathBuf {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir =
            std::env::temp_dir().join(format!("longplay-{name}-{}-{stamp}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn track(id: &str, path: &Path, duration: f64, pregap: f64, offset: f64) -> Track {
        Track {
            id: id.into(),
            number: 1,
            title: "T".into(),
            artist: "A".into(),
            duration,
            pregap,
            path: path.to_string_lossy().into_owned(),
            offset,
            side: "A".into(),
        }
    }

    #[test]
    fn decodes_wav_samples() {
        let dir = temp_dir("decode");
        let path = dir.join("tone.wav");
        write_wav(&path, 1.0, 44_100, 440.0);

        let decoded = decode_file(&path).unwrap();
        assert_eq!(decoded.rate, 44_100);
        assert_eq!(decoded.channels, 2);
        assert_eq!(decoded.samples.len(), 44_100 * 2);
        let peak = decoded
            .samples
            .iter()
            .fold(0.0f32, |acc, value| acc.max(value.abs()));
        assert!(peak > 0.3, "peak {peak}");

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn builds_a_side_with_gaps_and_markers() {
        let dir = temp_dir("side");
        let path = dir.join("tone.wav");
        write_wav(&path, 2.0, 44_100, 440.0);

        let side = Side {
            label: "A".into(),
            rpm: Rpm::R33,
            matrix: None,
            tracks: vec![
                track("a", &path, 1.0, 0.5, 0.0),
                track("b", &path, 1.0, 0.5, 0.5),
            ],
        };
        let timeline = SideTimeline::from_side(&side);
        let (audio, issues) = build_side(&side, &timeline, 48_000);

        assert!(issues.is_empty(), "{issues:?}");
        assert_eq!(audio.rate, 48_000);
        assert!((audio.duration - 3.0).abs() < 1e-3, "{}", audio.duration);
        assert_eq!(audio.tracks.len(), 2);
        assert_eq!(audio.tracks[0].start, 24_000);
        assert_eq!(audio.tracks[0].end, 72_000);
        assert_eq!(audio.tracks[1].start, 96_000);
        assert_eq!(audio.tracks[1].end, 144_000);

        // The pregap is silent, the audio is not.
        assert!(
            audio.samples[..24_000 * 2]
                .iter()
                .all(|value| *value == 0.0)
        );
        let peak = audio.samples[24_000 * 2..48_000 * 2]
            .iter()
            .fold(0.0f32, |acc, value| acc.max(value.abs()));
        assert!(peak > 0.2, "peak {peak}");

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn missing_files_become_silence_and_an_issue() {
        let side = Side {
            label: "A".into(),
            rpm: Rpm::R33,
            matrix: None,
            tracks: vec![track("a", Path::new("/nope/missing.flac"), 1.0, 0.0, 0.0)],
        };
        let timeline = SideTimeline::from_side(&side);
        let (audio, issues) = build_side(&side, &timeline, 48_000);
        assert_eq!(issues.len(), 1);
        assert!((audio.duration - 1.0).abs() < 1e-3);
        assert!(audio.samples.iter().all(|value| *value == 0.0));
    }
}
