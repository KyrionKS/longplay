//! Library index.
//!
//! Phase 1: folder scanning, tag reading via `lofty`, CUE parsing, artwork and
//! palette extraction, side detection and the SQLite cache.

pub mod artwork;
pub mod cue;
pub mod db;
pub mod metadata;
pub mod model;
pub mod palette;
pub mod scanner;
