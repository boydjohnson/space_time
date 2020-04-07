#![no_std]
#![feature(const_int_pow)]
#![feature(const_fn)]
#![deny(missing_docs)]

//! Port of https://github.com/locationtech/sfcurve scala space-filling curve library.
//!
//! Useful for representing and querying spatial objects

#[cfg(test)]
#[macro_use]
extern crate quickcheck_macros;

extern crate alloc;

pub mod index_range;
pub mod zorder;

use alloc::{boxed::Box, vec::Vec};
use index_range::IndexRange;
use zorder::z_curve_2d::ZCurve2D;

/// Factory providing space filling curves
pub struct SpaceFillingCurves;

impl SpaceFillingCurves {
    /// Return a `SpaceFillingCurve` type curve with a resolution.
    #[must_use]
    pub fn get_curve(curve: Curve, resolution: i32) -> impl SpaceFillingCurve2D {
        match curve {
            Curve::ZOrder => ZCurve2D::new(resolution),
            Curve::Hilbert => unimplemented!(),
        }
    }
}

/// The types of space-filling curves provided by the library.
#[derive(Debug, Clone, Copy, PartialEq)]
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

    /// Return an array-slice of `IndexRange`s.
    fn ranges(
        &self,
        x_min: f64,
        y_min: f64,
        x_max: f64,
        y_max: f64,
        hints: &[RangeComputeHints],
    ) -> Vec<Box<dyn IndexRange>>;
}

/// Hints to the `range` function implementation for `SpacefillingCurve2D`s.
pub enum RangeComputeHints {
    /// Number of times to recurse.
    MaxRecurse(usize),
}
