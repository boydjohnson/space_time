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
//! the SpaceFillingCurves factory and SpaceFillingCurve2D trait.
//! ```
//! use space_time::SpaceFillingCurves;
//!
//! let curve = SpaceFillingCurves::get_point_curve(1024);
//! let indexed_point = curve.index(2.3522, 48.8566);
//! let range_of_index = curve.ranges(2.35, 48.85, 2.354, 48.857, &[]);
//!
//! assert!(range_of_index
//!     .iter()
//!     .any(|r| r.lower() <= indexed_point && r.upper() >= indexed_point));
//! ```
//! Z3 curve is used for two dimensional point and time indexing.

pub mod binned_time;
pub mod index_range;
pub mod normalized_dimension;
pub mod zorder;

#[cfg(test)]
extern crate quickcheck;

#[cfg(test)]
#[macro_use]
extern crate quickcheck_macros;

extern crate alloc;

use zorder::z_curve_2d::ZCurve2D;

/// Factory providing space filling curves
pub struct SpaceFillingCurves;

impl SpaceFillingCurves {
    /// Return point indexing curve with a resolution.
    #[must_use]
    pub fn get_point_curve(resolution: i32) -> ZCurve2D {
        ZCurve2D::new(resolution)
    }
}

/// Hints to the `range` function implementation for `SpacefillingCurve2D`s.
pub enum RangeComputeHints {
    /// Number of times to recurse.
    MaxRecurse(usize),
}
