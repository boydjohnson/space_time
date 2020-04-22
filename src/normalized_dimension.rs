//! Normalizes an `f64` in the range from [MIN to MAX] to [0 to `MAX_INDEX`]
//!
//! `LatNormalizer` normalizes latitudes. [-90.0, 90].
//! `LonNormalizer` normalizes longitudes. [-180, 180].
//! `TimeNormalizer` normalizes floats in [0.0, MAX].

use core::convert::TryInto;

/// Maps a `f64` to an i32 <= `MAX_INDEX`.
pub trait NormalizedDimension {
    /// The minimum input.
    fn min(&self) -> f64;

    /// The maximum input.
    fn max(&self) -> f64;

    /// The maximum output value.
    fn max_index(&self) -> i32;

    /// Normalize input `f64` into range [0, `MAX_INDEX`].
    fn normalize(&self, x: f64) -> i32;

    /// Reverse of normalize.
    fn denormalize(&self, y: i32) -> f64;
}

/// A Range of doubles from `min` to `max`.
trait Range {
    /// The min of the range.
    fn min(&self) -> f64;

    /// The max of the range.
    fn max(&self) -> f64;
}

/// A helper trait to normalize a range of `f64`s to `i32` given a precision.
trait BitNormalizedDimension: Range {
    /// The precision of the normalizer.
    fn precision(&self) -> u8;

    /// Quantity used in `NormalizedDimension::normalize`.
    fn normalizer(&self) -> f64 {
        self.bins() as f64 / (self.max() - self.min())
    }

    /// Quantity used in `NormalizedDimension::denormalize`.
    fn denormalizer(&self) -> f64 {
        (self.max() - self.min()) / (self.bins() as f64)
    }

    /// Number of bins produced by the precision.
    fn bins(&self) -> i64 {
        1_i64 << self.precision()
    }
}

impl NormalizedDimension for dyn BitNormalizedDimension {
    fn min(&self) -> f64 {
        self.min()
    }

    fn max(&self) -> f64 {
        self.max()
    }

    fn max_index(&self) -> i32 {
        (self.bins() - 1)
            .try_into()
            .expect("Precision was restricted to allow bins - 1 to fit into i32")
    }

    fn normalize(&self, x: f64) -> i32 {
        if x >= self.max() {
            self.max_index()
        } else {
            ((x - self.min()) * self.normalizer()).floor() as i32
        }
    }

    fn denormalize(&self, y: i32) -> f64 {
        if y >= self.max_index() {
            self.min() + (f64::from(self.max_index()) + 0.5) * self.denormalizer() as f64
        } else {
            self.min() + (f64::from(y) + 0.5) * self.denormalizer() as f64
        }
    }
}

/// Normalize an input with a `BitNormalizedDimension` normalizer.
fn normalize<T: BitNormalizedDimension + 'static>(normalizer: &T, x: f64) -> i32 {
    <dyn BitNormalizedDimension as NormalizedDimension>::normalize(normalizer, x)
}

/// Denormalize an input with a `BitNormalizedDimension` denormalizer.
fn denormalize<T: BitNormalizedDimension + 'static>(normalizer: &T, y: i32) -> f64 {
    <dyn BitNormalizedDimension as NormalizedDimension>::denormalize(normalizer, y)
}

fn max_index<T: BitNormalizedDimension + 'static>(normalizer: &T) -> i32 {
    <dyn BitNormalizedDimension as NormalizedDimension>::max_index(normalizer)
}

/// A `NormalizedDimension` for Latitudes.
#[derive(Debug, PartialEq)]
pub struct LatNormalizer {
    precision: u8,
}

impl LatNormalizer {
    /// Constructor panics if precision is too high (> 31) or 0.
    #[must_use]
    pub fn new(precision: u8) -> Self {
        assert!(precision > 0);
        assert!(precision <= 31);
        LatNormalizer { precision }
    }
}

/// A `NormalizedDimension` for Longitudes.
#[derive(Debug, PartialEq)]
pub struct LonNormalizer {
    precision: u8,
}

impl LonNormalizer {
    /// Constructor panics if precision is too high (> 31) or 0.
    #[must_use]
    pub fn new(precision: u8) -> Self {
        assert!(precision > 0);
        assert!(precision <= 31);
        LonNormalizer { precision }
    }
}

/// A `NormalizedDimension` for time.
#[derive(Debug, PartialEq)]
pub struct TimeNormalizer {
    precision: u8,
    max: f64,
}

impl TimeNormalizer {
    /// Constructor returns None if precision is too high or 0.
    #[must_use]
    pub fn new(precision: u8, max: f64) -> Self {
        assert!(precision > 0);
        assert!(precision <= 31);
        TimeNormalizer { precision, max }
    }
}

impl Range for LatNormalizer {
    fn min(&self) -> f64 {
        -90.0
    }

    fn max(&self) -> f64 {
        90.0
    }
}

impl Range for LonNormalizer {
    fn min(&self) -> f64 {
        -180.0
    }

    fn max(&self) -> f64 {
        180.0
    }
}

impl Range for TimeNormalizer {
    fn min(&self) -> f64 {
        0.0
    }

    fn max(&self) -> f64 {
        self.max
    }
}

impl BitNormalizedDimension for LatNormalizer {
    fn precision(&self) -> u8 {
        self.precision
    }
}

impl BitNormalizedDimension for LonNormalizer {
    fn precision(&self) -> u8 {
        self.precision
    }
}

impl BitNormalizedDimension for TimeNormalizer {
    fn precision(&self) -> u8 {
        self.precision
    }
}

impl NormalizedDimension for LatNormalizer {
    fn min(&self) -> f64 {
        <Self as Range>::min(&self)
    }

    fn max(&self) -> f64 {
        <Self as Range>::max(&self)
    }

    fn max_index(&self) -> i32 {
        max_index(self)
    }

    fn normalize(&self, x: f64) -> i32 {
        normalize(self, x)
    }

    fn denormalize(&self, y: i32) -> f64 {
        denormalize(self, y)
    }
}

impl NormalizedDimension for LonNormalizer {
    fn min(&self) -> f64 {
        <Self as Range>::min(&self)
    }

    fn max(&self) -> f64 {
        <Self as Range>::max(&self)
    }

    fn max_index(&self) -> i32 {
        max_index(self)
    }

    fn normalize(&self, x: f64) -> i32 {
        normalize(self, x)
    }

    fn denormalize(&self, y: i32) -> f64 {
        denormalize(self, y)
    }
}

impl NormalizedDimension for TimeNormalizer {
    fn min(&self) -> f64 {
        <Self as Range>::min(&self)
    }

    fn max(&self) -> f64 {
        <Self as Range>::max(&self)
    }

    fn max_index(&self) -> i32 {
        max_index(self)
    }

    fn normalize(&self, x: f64) -> i32 {
        normalize(self, x)
    }

    fn denormalize(&self, y: i32) -> f64 {
        denormalize(self, y)
    }
}

#[cfg(test)]
mod tests {

    use super::{LatNormalizer, LonNormalizer, NormalizedDimension};

    #[test]
    fn test_normalize_round_trip_minimum() {
        let norm_lat = LatNormalizer::new(31);
        let norm_lon = LonNormalizer::new(31);

        assert_eq!(norm_lat.normalize(norm_lat.denormalize(0)), 0);
        assert_eq!(norm_lon.normalize(norm_lon.denormalize(0)), 0);
    }

    #[test]
    fn test_normalize_round_trip_maximum() {
        let norm_lat = LatNormalizer::new(31);
        let norm_lon = LonNormalizer::new(31);
        let max_bin = (2_i64.pow(31) - 1) as i32;
        assert_eq!(norm_lat.normalize(norm_lat.denormalize(max_bin)), max_bin);
        assert_eq!(norm_lon.normalize(norm_lon.denormalize(max_bin)), max_bin);
    }

    #[test]
    fn test_normalize_min() {
        let norm_lat = LatNormalizer::new(31);
        let norm_lon = LonNormalizer::new(31);

        assert_eq!(norm_lat.normalize(norm_lat.min()), 0);
        assert_eq!(norm_lon.normalize(norm_lon.min()), 0)
    }

    #[test]
    fn test_normalize_max() {
        let norm_lat = LatNormalizer::new(31);
        let norm_lon = LonNormalizer::new(31);
        let max_bin = (2_i64.pow(31) - 1) as i32;

        assert_eq!(norm_lat.normalize(norm_lat.max()), max_bin);
        assert_eq!(norm_lon.normalize(norm_lon.max()), max_bin);
    }

    #[test]
    fn test_denormalize_to_middle() {
        let norm_lat = LatNormalizer::new(31);
        let norm_lon = LonNormalizer::new(31);
        let max_bin = (2_i64.pow(31) - 1) as i32;

        let lat_extent = norm_lat.max() - norm_lat.min();
        let lon_extent = norm_lon.max() - norm_lon.min();
        let lat_width = lat_extent / (max_bin as f64 + 1_f64);
        let lon_width = lon_extent / (max_bin as f64 + 1_f64);

        assert_eq!(norm_lat.denormalize(0), norm_lat.min() + lat_width / 2.0);
        assert_eq!(
            norm_lat.denormalize(max_bin),
            norm_lat.max() - lat_width / 2.0
        );

        assert_eq!(norm_lon.denormalize(0), norm_lon.min() + lon_width / 2.0);
        assert_eq!(
            norm_lon.denormalize(max_bin),
            norm_lon.max() - lon_width / 2.0
        );
    }
}
