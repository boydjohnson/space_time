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

//! Implementation of `SpaceFillingCurve2D` for zorder.

use crate::index_range::IndexRange;
use crate::zorder::{z_2::Z2, z_n::ZN, z_range::ZRange};
use crate::RangeComputeHints;
use alloc::{boxed::Box, vec::Vec};

/// 2-Dimensional `ZCurve`, with x as longitude and y as latitude.
pub struct ZCurve2D {
    resolution: u32,
    x_min: f64,
    x_max: f64,
    y_min: f64,
    y_max: f64,
}

impl Default for ZCurve2D {
    fn default() -> Self {
        ZCurve2D {
            resolution: 1024,
            x_min: -180.0,
            x_max: 180.0,
            y_min: -90.0,
            y_max: 90.0,
        }
    }
}

impl ZCurve2D {
    /// Max Recursion constant to use.
    const MAX_RECURSION: usize = 32;

    /// Constructor.
    #[must_use]
    pub fn new(resolution: u32, x_min: f64, y_min: f64, x_max: f64, y_max: f64) -> Self {
        ZCurve2D {
            resolution,
            x_min,
            x_max,
            y_min,
            y_max,
        }
    }

    fn cell_width(&self) -> f64 {
        (self.x_max - self.x_min) / f64::from(self.resolution)
    }

    fn cell_height(&self) -> f64 {
        (self.y_max - self.y_min) / f64::from(self.resolution)
    }

    fn map_to_col(&self, x: f64) -> u32 {
        ((x - self.x_min) / self.cell_width()) as u32
    }

    fn map_to_row(&self, y: f64) -> u32 {
        ((self.y_max - y) / self.cell_height()) as u32
    }

    fn col_to_map(&self, col: u32) -> f64 {
        (f64::from(col) * self.cell_width() + self.x_min + self.cell_width() / 2.0)
            .min(self.x_max)
            .max(self.x_min)
    }

    fn row_to_map(&self, row: u32) -> f64 {
        (self.y_max - f64::from(row) * self.cell_height() - self.cell_height() / 2.0)
            .max(self.y_min)
            .min(self.y_max)
    }

    /// Get the index for a point.
    #[must_use]
    pub fn index(&self, x: f64, y: f64) -> u64 {
        let col = self.map_to_col(x);
        let row = self.map_to_row(y);
        Z2::new(col, row).z()
    }

    /// Get the point for an index.
    #[must_use]
    pub fn point(&self, index: u64) -> (f64, f64) {
        let (col, row) = Z2::new_from_zorder(index).decode();
        (self.col_to_map(col), self.row_to_map(row))
    }

    /// Get the index ranges for a bounding box.
    #[must_use]
    pub fn ranges(
        &self,
        x_min: f64,
        y_min: f64,
        x_max: f64,
        y_max: f64,
        hints: &[RangeComputeHints],
    ) -> Vec<Box<dyn IndexRange>> {
        let col_min = self.map_to_col(x_min);
        let row_min = self.map_to_row(y_max);
        let min = Z2::new(col_min, row_min);

        let col_max = self.map_to_col(x_max);
        let row_max = self.map_to_row(y_min);
        let max = Z2::new(col_max, row_max);

        let max_recurse = hints.iter().find_map(|h| {
            let RangeComputeHints::MaxRecurse(max) = *h;
            if max > Self::MAX_RECURSION {
                Some(Self::MAX_RECURSION)
            } else {
                Some(max)
            }
        });

        Z2::zranges::<Z2>(
            &[ZRange {
                min: min.z(),
                max: max.z(),
            }],
            64,
            None,
            max_recurse,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SpaceFillingCurves;

    #[test]
    fn test_produce_covering_ranges() {
        let curve = SpaceFillingCurves::get_point_curve(1024, -180.0, -90.0, 180.0, 90.0);

        let ranges = curve.ranges(
            -80.0,
            35.0,
            -75.0,
            40.0,
            &[RangeComputeHints::MaxRecurse(32)],
        );

        assert_eq!(ranges.len(), 44);

        let (l, r, contains) = ranges[0].tuple();
        assert_eq!(l, 197616);
        assert_eq!(r, 197631);
        assert!(contains);
    }

    #[test]
    fn test_col_to_map_map_to_col() {
        let curve = ZCurve2D::default();
        let m = curve.col_to_map(27);
        let col = curve.map_to_col(m);
        assert_eq!(col, 27);
    }

    #[test]
    fn point_to_index_to_point() {
        let curve = ZCurve2D::default();
        let index = curve.index(-45.0, -45.0);
        let point = curve.point(index);
        assert!(point > (-45.0 - 1.0, -45.0 - 1.0));
        assert!(point < (-45.0 + 1.0, -45.0 + 1.0));
    }

    #[test]
    fn test_sweep_through_map() {
        let curve = ZCurve2D::default();

        let mut lon = -180.0;
        let mut lat = -90.0;

        while lon < 180.0 {
            while lat < 90.0 {
                let indexed_point = curve.index(lon, lat);
                let range = curve.ranges(
                    (lon - 10.0).max(-180.0),
                    (lat - 10.0).max(-90.0),
                    (lon + 10.0).min(180.0),
                    (lat + 10.0).min(90.0),
                    &[],
                );
                assert!(range
                    .iter()
                    .any(|r| r.lower() <= indexed_point && indexed_point <= r.upper()));

                lat += 1.0;
            }
            lon += 1.0;
        }
    }
}
