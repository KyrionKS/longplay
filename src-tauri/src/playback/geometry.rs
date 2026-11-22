//! Geometry of a pivot tonearm.
//!
//! Spindle at the origin, pivot at `(pivot_distance, 0)`. The stylus sits at
//! groove radius `r`; the arm angle `theta` is measured at the pivot between
//! the pivot→spindle line and the pivot→stylus line.
//!
//! `cos(theta) = (d² + L² − r²) / (2·d·L)`

use std::f64::consts::PI;

/// Physical dimensions of a tonearm, in millimetres and radians.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TonearmGeometry {
    /// Distance from spindle to pivot.
    pub pivot_distance: f64,
    /// Effective arm length (pivot to stylus).
    pub effective_length: f64,
    /// Groove radius at the start of the side (outer edge).
    pub outer_radius: f64,
    /// Groove radius at the run-out (near the label).
    pub inner_radius: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeometryError {
    /// A dimension is zero or negative.
    NonPositiveDimension,
    /// The inner radius is not smaller than the outer radius.
    InvalidRadiusRange,
    /// The arm length cannot reach both the inner and the outer groove.
    UnreachableGroove,
}

impl TonearmGeometry {
    /// 9-inch arm on a 12" LP.
    pub const LP12: Self = Self {
        pivot_distance: 211.0,
        effective_length: 230.0,
        outer_radius: 146.0,
        inner_radius: 60.0,
    };

    /// 9-inch arm on a 7" single.
    pub const SINGLE7: Self = Self {
        pivot_distance: 211.0,
        effective_length: 230.0,
        outer_radius: 87.5,
        inner_radius: 37.0,
    };

    pub fn new(
        pivot_distance: f64,
        effective_length: f64,
        outer_radius: f64,
        inner_radius: f64,
    ) -> Result<Self, GeometryError> {
        let geometry = Self {
            pivot_distance,
            effective_length,
            outer_radius,
            inner_radius,
        };
        geometry.validate()?;
        Ok(geometry)
    }

    pub fn validate(&self) -> Result<(), GeometryError> {
        if self.pivot_distance <= 0.0
            || self.effective_length <= 0.0
            || self.outer_radius <= 0.0
            || self.inner_radius <= 0.0
        {
            return Err(GeometryError::NonPositiveDimension);
        }
        if !self.inner_radius.is_finite() || self.inner_radius >= self.outer_radius {
            return Err(GeometryError::InvalidRadiusRange);
        }
        for radius in [self.inner_radius, self.outer_radius] {
            if !self.reaches(radius) {
                return Err(GeometryError::UnreachableGroove);
            }
        }
        Ok(())
    }

    fn reaches(&self, radius: f64) -> bool {
        let shortest = (self.pivot_distance - radius).abs();
        let longest = self.pivot_distance + radius;
        self.effective_length >= shortest - f64::EPSILON
            && self.effective_length <= longest + f64::EPSILON
    }

    pub fn clamp_radius(&self, radius: f64) -> f64 {
        radius.clamp(self.inner_radius, self.outer_radius)
    }

    /// Radius for a playback progress of the side, `0.0` = outer, `1.0` = inner.
    pub fn radius_for_progress(&self, progress: f64) -> f64 {
        let p = progress.clamp(0.0, 1.0);
        self.outer_radius - (self.outer_radius - self.inner_radius) * p
    }

    /// Inverse of [`Self::radius_for_progress`].
    pub fn progress_for_radius(&self, radius: f64) -> f64 {
        let r = self.clamp_radius(radius);
        (self.outer_radius - r) / (self.outer_radius - self.inner_radius)
    }

    /// Arm angle in radians for a stylus at `radius`.
    pub fn angle_for_radius(&self, radius: f64) -> f64 {
        let r = self.clamp_radius(radius);
        let cosine = (self.pivot_distance.powi(2) + self.effective_length.powi(2) - r.powi(2))
            / (2.0 * self.pivot_distance * self.effective_length);
        cosine.clamp(-1.0, 1.0).acos()
    }

    pub fn angle_for_progress(&self, progress: f64) -> f64 {
        self.angle_for_radius(self.radius_for_progress(progress))
    }

    /// Stylus radius for a given arm angle, clamped to the groove range.
    pub fn radius_for_angle(&self, angle: f64) -> f64 {
        let theta = angle.clamp(0.0, PI);
        let squared = self.pivot_distance.powi(2) + self.effective_length.powi(2)
            - 2.0 * self.pivot_distance * self.effective_length * theta.cos();
        self.clamp_radius(squared.max(0.0).sqrt())
    }

    /// Angular sweep from the outer groove to the run-out, in radians.
    pub fn sweep(&self) -> f64 {
        self.angle_for_radius(self.outer_radius) - self.angle_for_radius(self.inner_radius)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f64 = 1e-3;

    #[test]
    fn matches_known_arm_angles() {
        let arm = TonearmGeometry::LP12;
        // 38.35° at the outer groove, 14.84° near the label.
        assert!((arm.angle_for_radius(arm.outer_radius) - 0.6693).abs() < EPS);
        assert!((arm.angle_for_radius(arm.inner_radius) - 0.2590).abs() < EPS);
        assert!((arm.sweep() - 0.4103).abs() < EPS);
    }

    #[test]
    fn angle_decreases_towards_the_label() {
        let arm = TonearmGeometry::LP12;
        let mut previous = arm.angle_for_progress(0.0);
        for step in 1..=20 {
            let angle = arm.angle_for_progress(step as f64 / 20.0);
            assert!(angle < previous, "angle must shrink towards the label");
            previous = angle;
        }
    }

    #[test]
    fn progress_and_radius_round_trip() {
        let arm = TonearmGeometry::LP12;
        for step in 0..=100 {
            let progress = step as f64 / 100.0;
            let radius = arm.radius_for_progress(progress);
            assert!((arm.progress_for_radius(radius) - progress).abs() < 1e-9);
            let angle = arm.angle_for_radius(radius);
            let back = arm.radius_for_angle(angle);
            assert!((back - radius).abs() < 1e-6);
        }
    }

    #[test]
    fn progress_is_clamped() {
        let arm = TonearmGeometry::LP12;
        assert_eq!(arm.radius_for_progress(-1.0), arm.outer_radius);
        assert_eq!(arm.radius_for_progress(2.0), arm.inner_radius);
        assert_eq!(arm.progress_for_radius(0.0), 1.0);
        assert_eq!(arm.progress_for_radius(1000.0), 0.0);
    }

    #[test]
    fn rejects_impossible_geometry() {
        assert_eq!(
            TonearmGeometry::new(211.0, 100.0, 146.0, 60.0),
            Err(GeometryError::UnreachableGroove)
        );
        assert_eq!(
            TonearmGeometry::new(211.0, 230.0, 60.0, 146.0),
            Err(GeometryError::InvalidRadiusRange)
        );
        assert_eq!(
            TonearmGeometry::new(0.0, 230.0, 146.0, 60.0),
            Err(GeometryError::NonPositiveDimension)
        );
        assert!(TonearmGeometry::new(211.0, 230.0, 146.0, 60.0).is_ok());
    }

    #[test]
    fn single_uses_a_smaller_groove_range() {
        let single = TonearmGeometry::SINGLE7;
        assert!(single.sweep() > 0.0);
        assert!(single.radius_for_progress(0.0) < TonearmGeometry::LP12.outer_radius);
    }
}
