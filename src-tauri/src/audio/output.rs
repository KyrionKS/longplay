//! Platform audio output.
//!
//! The stream is created and owned by a dedicated thread: `cpal::Stream` is not
//! `Send` on every backend, and parking the thread keeps it alive for the app's
//! lifetime. On Linux (development without ALSA headers) a real-time paced null
//! sink drives the engine so the UI and position still work.

use std::sync::Arc;
use std::time::Duration;

use super::engine::Engine;

#[cfg(not(target_os = "linux"))]
pub fn start(engine: Arc<Engine>) -> Result<u32, String> {
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let built = (|| -> Result<(cpal::Stream, u32), String> {
            let host = cpal::default_host();
            let device = host
                .default_output_device()
                .ok_or_else(|| "kein Ausgabegerät gefunden".to_string())?;
            let mut supported = device
                .default_output_config()
                .map_err(|error| error.to_string())?;

            // CoreAudio exposes f32 for virtually every device. If the default
            // config uses another format, look for an f32 config at the same
            // rate instead of failing.
            if supported.sample_format() != cpal::SampleFormat::F32 {
                let rate = supported.sample_rate();
                let fallback = device
                    .supported_output_configs()
                    .map_err(|error| error.to_string())?
                    .find(|range| range.sample_format() == cpal::SampleFormat::F32)
                    .map(|range| range.with_sample_rate(rate));
                match fallback {
                    Some(config) => supported = config,
                    None => {
                        return Err(format!(
                            "Ausgabegerät liefert {:?}, kein f32-Ausgang verfügbar",
                            supported.sample_format()
                        ));
                    }
                }
            }

            let channels = usize::from(supported.channels());
            if channels == 0 {
                return Err("Ausgabegerät meldet keine Kanäle".to_string());
            }

            let config = supported.config();
            let rate = config.sample_rate;
            let name = device.name().unwrap_or_default();
            eprintln!("Audio: {name}, {channels} Kanäle, {rate} Hz");

            let mut dsp = super::engine::DspState::new();
            let mut scratch = Vec::new();
            let stream = device
                .build_output_stream(
                    config,
                    move |data: &mut [f32], _| {
                        engine.process_device(data, channels, rate, &mut dsp, &mut scratch);
                    },
                    |error| eprintln!("Audio-Ausgabe: {error}"),
                    None,
                )
                .map_err(|error| error.to_string())?;
            stream.play().map_err(|error| error.to_string())?;
            Ok((stream, rate))
        })();

        match built {
            Ok((stream, rate)) => {
                let _ = sender.send(Ok(rate));
                // Keep the stream alive for the process lifetime.
                let _stream = stream;
                loop {
                    std::thread::park();
                }
            }
            Err(error) => {
                let _ = sender.send(Err(error));
            }
        }
    });

    receiver
        .recv_timeout(Duration::from_secs(5))
        .map_err(|_| "Audio-Ausgabe antwortet nicht".to_string())?
}

#[cfg(target_os = "linux")]
pub fn start(engine: Arc<Engine>) -> Result<u32, String> {
    let rate = 48_000;
    std::thread::spawn(move || {
        let mut dsp = super::engine::DspState::new();
        let mut scratch = Vec::new();
        let mut buffer = vec![0.0f32; 480 * 2];
        loop {
            engine.process_device(&mut buffer, 2, rate, &mut dsp, &mut scratch);
            std::thread::sleep(Duration::from_millis(10));
        }
    });
    Ok(rate)
}
