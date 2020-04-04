#![no_std]
#![deny(missing_docs)]

//! Port of https://github.com/locationtech/sfcurve scala space-filling curve library.
//!
//! Useful for representing and querying spatial objects

pub mod index_range;

use index_range::IndexRange;

/// Factory providing space filling curves
pub struct SpaceFillingCurves;

// impl SpaceFillingCurves {
//     /// Return a SpaceFillingCurve type curve with a pricision.
//     pub fn get_curve(curve: Curve, precision: i32) -> impl SpaceFillingCurve2D {
//         unimplemented!();
//     }
// }

/// The types of space-filling curves provided by the library.
pub enum Curve {
    /// Z-Order curve.
    ZOrder,
    /// Hilbert curve.
    Hilbert,
}

/// Trait for all space-filling curves.
pub trait SpaceFillingCurve2D {
    /// Return the index of a point.
    fn index(&self, x: f64, y: f64) -> i64;

    /// Return a point from an index.
    fn point(&self, index: i64) -> (f64, f64);

    /// Return an array-slice of IndexRanges.
    fn ranges(
        &self,
        x_min: f64,
        y_min: f64,
        x_max: f64,
        y_max: f64,
        hints: Option<&[RangeComputeHints]>,
    ) -> &[&dyn IndexRange];
}

/// Hints to the `range` function implementation for `SpacefillingCurve2D`s.
pub enum RangeComputeHints {
    /// Number of times to recurse.
    MaxRecurse(usize),
}
