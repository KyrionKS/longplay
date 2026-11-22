//! Audio pipeline.
//!
//! Phase 2: symphonia decoder on a worker thread, a 45 s ring buffer, a
//! fractional resampler driven by the platter rate, procedural vinyl noise and
//! the cpal output stream.
//!
//! The output device lives behind the `audio-device` feature so the DSP can be
//! built and tested on machines without ALSA headers.
