#![no_std]
#![feature(trait_alias)]
#![feature(const_int_pow)]
#![feature(const_fn)]
#![deny(missing_docs)]
//! Partial port of the scala-based geomesa-z3 library from [geomesa](http://github.com/locationtech/geomesa)
//! Partial port of [sfcurve](https://github.com/locationtech/sfcurve) scala space-filling curve library.
//!
//! Useful for representing and querying spatial objects
//!
//! Z2 curve is used for two dimensional point indexing and can be accessed through
//! the `SpaceFillingCurves` factory.
//! ```
//! use space_time::SpaceFillingCurves;
//!
//! let curve = SpaceFillingCurves::get_point_curve(1024, -180.0, -90.0, 180.0, 90.0);
//! let indexed_point = curve.index(2.3522, 48.8566);
//! let range_of_index = curve.ranges(2.35, 48.85, 2.354, 48.857, &[]);
//!
//! assert!(range_of_index
//!     .iter()
//!     .any(|r| r.lower() <= indexed_point && r.upper() >= indexed_point));
//! ```
//! Z3 curve is used for two dimensional point and time indexing and can be accessed
//! through the `SpaceTimeFillingCurves` factory.
//! ```
//! use space_time::SpaceTimeFillingCurves;
//!
//! let curve = SpaceTimeFillingCurves::get_point_curve(1024, -180.0, -90.0, 180.0, 90.0, 159753997829.0);
//! let indexed_point_in_time = curve.index(2.3522, 48.8566, 1587583997829.0); // Paris, France. April 22, 2020 as milliseconds since Unix Epoch.
//! let range_of_index = curve.ranges(2.3522, 48.85, 2.354, 48.857, 1587583997828.0, 1587583997828.0, &[]);
//!
//! assert!(range_of_index.iter().any(|r| r.lower() <= indexed_point_in_time && r.upper() >= indexed_point_in_time));
//! ```

pub mod index_range;
pub mod zorder;

#[cfg(test)]
extern crate quickcheck;

#[cfg(test)]
#[macro_use]
extern crate quickcheck_macros;

extern crate alloc;

// use zorder::z_3::Z3TimeCurve;
use zorder::z_3::ZCurve3D;
use zorder::z_curve_2d::ZCurve2D;

/// Factory providing space filling curves
pub struct SpaceFillingCurves;

impl SpaceFillingCurves {
    /// Return point indexing curve with a resolution.
    #[must_use]
    pub fn get_point_curve(
        resolution: u32,
        x_min: f64,
        y_min: f64,
        x_max: f64,
        y_max: f64,
    ) -> ZCurve2D {
        ZCurve2D::new(resolution, x_min, y_min, x_max, y_max)
    }
}

/// Factory providing space-time filling curves
pub struct SpaceTimeFillingCurves;

impl SpaceTimeFillingCurves {
    /// Return point-time indexing curve.
    #[must_use]
    pub fn get_point_curve(
        resolution: u32,
        x_min: f64,
        y_min: f64,
        x_max: f64,
        y_max: f64,
        t_max: f64,
    ) -> ZCurve3D {
        ZCurve3D::new(resolution, x_min, y_min, x_max, y_max, t_max)
    }
}

/// Hints to the `range` function implementation for `SpacefillingCurve2D`s.
pub enum RangeComputeHints {
    /// Number of times to recurse.
    MaxRecurse(usize),
}
