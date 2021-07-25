//
// Copyright 2020, Gobsmacked Labs, LLC.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#![no_std]
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
//!
//! Extended Z-order curves are used for non-points.
//! `XZ2SFC` for spatial indexing of non-points.
//! ```
//! use space_time::SpaceFillingCurves;
//!
//! let curve = SpaceFillingCurves::get_non_point_curve(12, -180.0, -90.0, 180.0, 90.0);
//! let indexed_polygon = curve.index(2.3522, 48.8466, 2.39, 49.9325);
//! let range_of_index = curve.ranges(2.0, 48.0, 3.0, 50.0, None);
//!
//! assert!(range_of_index
//!     .iter()
//!     .any(|r| r.lower() <= indexed_polygon && r.upper() >= indexed_polygon));
//! ```
//! `XZ3SFC` for spatial-temporal indexing of non-points.
//!
//! ```
//! use space_time::SpaceTimeFillingCurves;
//!
//! let curve = SpaceTimeFillingCurves::get_non_point_curve(
//!     12,
//!     -180.0,
//!     -90.0,
//!     0.0,
//!     180.0,
//!     90.0,
//!     1_893_456_000.0,
//! );
//!
//! let indexed_polygon = curve.index(
//!     2.3522,
//!     48.8466,
//!     1_556_496_000.0,
//!     2.39,
//!     49.9325,
//!     1_556_496_000.0,
//! );
//!
//! let range_of_index = curve.ranges(2.0, 48.0, 1_556_300_000.0, 3.0, 50.0, 1_557_496_000.0, None);
//!
//! assert!(range_of_index
//!     .iter()
//!     .any(|r| r.lower() <= indexed_polygon && r.upper() >= indexed_polygon));
//! ```

pub mod index_range;
pub mod xzorder;
pub mod zorder;

#[cfg(test)]
extern crate quickcheck;

#[cfg(test)]
#[macro_use]
extern crate quickcheck_macros;

extern crate alloc;

use xzorder::xz2_sfc::XZ2SFC;
use xzorder::xz3_sfc::XZ3SFC;
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

    /// Return a non-point indexing curve with a resolution.
    #[must_use]
    pub fn get_non_point_curve(
        resolution: u32,
        x_min: f64,
        y_min: f64,
        x_max: f64,
        y_max: f64,
    ) -> XZ2SFC {
        XZ2SFC::new(resolution, x_min, y_min, x_max, y_max)
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

    /// Return a nonpoint space-time indexing curve.
    #[must_use]
    pub fn get_non_point_curve(
        resolution: u32,
        x_min: f64,
        y_min: f64,
        z_min: f64,
        x_max: f64,
        y_max: f64,
        z_max: f64,
    ) -> XZ3SFC {
        XZ3SFC::new(resolution, x_min, y_min, z_min, x_max, y_max, z_max)
    }
}

/// Hints to the `range` function implementation for `SpacefillingCurve2D`s.
pub enum RangeComputeHints {
    /// Number of times to recurse.
    MaxRecurse(usize),
}
