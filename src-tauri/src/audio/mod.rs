//! Audio pipeline.
//!
//! Decoding, interpolation, procedural vinyl noise and platform output. A whole
//! side is decoded into one 48 kHz stereo buffer, which keeps playback, seeking
//! and scratching sample accurate and free of locks in the audio callback.

pub mod decode;
pub mod engine;
pub mod noise;
pub mod output;
pub mod resample;
pub mod session;
pub mod timeline;

pub use engine::{DspState, Engine};
pub use session::{PlaybackState, Session};
pub use timeline::{SideTimeline, TrackPosition, TrackSpan};
