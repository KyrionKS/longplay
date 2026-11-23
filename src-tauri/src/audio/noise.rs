//! Procedural vinyl surface noise: crackle, rumble, needle drop.

use std::f32::consts::TAU;

pub struct VinylNoise {
    rng: u32,
    crackle: f32,
    hp_prev: f32,
    rumble: f32,
    rumble_lp: f32,
    drop: f32,
    drop_phase: f32,
    tick: f32,
    surface: f32,
}

impl VinylNoise {
    pub fn new(seed: u32) -> Self {
        Self {
            rng: seed | 1,
            crackle: 0.0,
            hp_prev: 0.0,
            rumble: 0.0,
            rumble_lp: 0.0,
            drop: 0.0,
            drop_phase: 0.0,
            tick: 0.0,
            surface: 0.0,
        }
    }

    fn next(&mut self) -> f32 {
        let mut x = self.rng;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.rng = x;
        (x >> 8) as f32 / 16_777_216.0
    }

    /// Arms the needle-drop thump.
    pub fn trigger_drop(&mut self) {
        self.drop = 1.0;
        self.drop_phase = 0.0;
        self.tick = 1.0;
    }

    /// Adds a short, denser crackle bed, as when the needle just touched down.
    pub fn trigger_surface(&mut self, seconds: f32) {
        self.surface = seconds.max(0.0);
    }

    /// Returns one stereo noise frame. `character` is `0.0..=1.0`.
    pub fn process(&mut self, character: f32, rate: u32, platter: f32) -> (f32, f32) {
        if character <= 0.0 {
            return (0.0, 0.0);
        }

        let rate = rate.max(1) as f64;
        let character = character.clamp(0.0, 1.0);
        let platter = platter.clamp(0.0, 1.0);

        // ~8 crackles per minute in steady state; after the needle drops a
        // short burst of ~22 crackles per second makes the touchdown audible.
        let (density, amplitude) = if self.surface > 0.0 {
            (28.0 / rate, 0.26)
        } else {
            (8.0 / (60.0 * rate), 0.10)
        };
        if f64::from(self.next()) < density {
            self.crackle = (0.15 + 0.85 * self.next()) * amplitude;
        }
        if self.surface > 0.0 {
            self.surface = (self.surface - 1.0 / rate as f32).max(0.0);
        }

        let mut crackle = 0.0;
        if self.crackle > 1e-4 {
            let white = self.next() * 2.0 - 1.0;
            let highpass = white - self.hp_prev;
            self.hp_prev = white;
            crackle = highpass * self.crackle;
            self.crackle *= 0.995;
        }

        let white = self.next() * 2.0 - 1.0;
        self.rumble = (self.rumble + white * 0.02).clamp(-1.0, 1.0);
        self.rumble_lp += (self.rumble - self.rumble_lp) * 0.004;
        let rumble = self.rumble_lp * 0.04 * platter;

        let mut drop = 0.0;
        if self.drop > 1e-4 {
            drop = (self.drop_phase * TAU).sin() * self.drop * 0.55;
            self.drop_phase = (self.drop_phase + 55.0 / rate as f32) % 1.0;
            self.drop *= 0.9995;
        }

        let mut tick = 0.0;
        if self.tick > 1e-4 {
            tick = (self.next() * 2.0 - 1.0) * self.tick * 0.18;
            self.tick *= 0.985;
        }

        let signal = (crackle + rumble + drop + tick) * character;
        (signal, signal * 0.85)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn character_zero_is_silent() {
        let mut noise = VinylNoise::new(7);
        for _ in 0..2_000 {
            assert_eq!(noise.process(0.0, 48_000, 1.0), (0.0, 0.0));
        }
    }

    #[test]
    fn is_deterministic_for_a_seed() {
        let mut a = VinylNoise::new(42);
        let mut b = VinylNoise::new(42);
        for _ in 0..5_000 {
            assert_eq!(a.process(0.7, 48_000, 1.0), b.process(0.7, 48_000, 1.0));
        }
    }

    #[test]
    fn needle_drop_decays() {
        let mut noise = VinylNoise::new(11);
        noise.trigger_drop();
        let mut first = 0.0;
        let mut later = 0.0;
        for index in 0..48_000 {
            let (left, _) = noise.process(1.0, 48_000, 0.0);
            if index < 4_800 {
                first += left.abs();
            } else if index > 24_000 {
                later += left.abs();
            }
        }
        assert!(first > later * 4.0, "{first} vs {later}");
    }

    #[test]
    fn never_produces_nan() {
        let mut noise = VinylNoise::new(3);
        for index in 0..10_000 {
            let (left, right) = noise.process(1.0, 44_100, (index % 100) as f32 / 100.0);
            assert!(left.is_finite() && right.is_finite());
        }
    }
}
