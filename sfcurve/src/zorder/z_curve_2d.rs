//! Implementation of `SpaceFillingCurve2D` for zorder.

use crate::index_range::IndexRange;
use crate::zorder::{z_2::Z2, z_n::ZN, z_range::ZRange};
use crate::{RangeComputeHints, SpaceFillingCurve2D};
use alloc::{boxed::Box, vec::Vec};

/// 2-Dimensional `ZCurve`, with x as longitude and y as latitude.
pub struct ZCurve2D {
    resolution: i32,
}

impl ZCurve2D {
    /// Constructor.
    pub fn new(resolution: i32) -> Self {
        ZCurve2D { resolution }
    }

    fn cell_width(&self) -> f64 {
        (Self::X_MAX - Self::X_MIN) / self.resolution as f64
    }

    fn cell_height(&self) -> f64 {
        (Self::Y_MAX - Self::Y_MIN) / self.resolution as f64
    }

    fn map_to_col(&self, x: f64) -> i32 {
        ((x - Self::X_MIN) / self.cell_width()) as i32
    }

    fn map_to_row(&self, y: f64) -> i32 {
        ((Self::Y_MAX - y) / self.cell_height()) as i32
    }

    fn col_to_map(&self, col: i32) -> f64 {
        (col as f64 * self.cell_width() + Self::X_MIN + self.cell_width() / 2.0)
            .min(Self::X_MAX)
            .max(Self::X_MIN)
    }

    fn row_to_map(&self, row: i32) -> f64 {
        (Self::Y_MAX - row as f64 * self.cell_height() - self.cell_height() / 2.0)
            .max(Self::Y_MIN)
            .min(Self::Y_MAX)
    }

    /// min long
    const X_MIN: f64 = -180.0;
    /// min lat
    const Y_MIN: f64 = -90.0;
    /// max long
    const X_MAX: f64 = 180.0;
    /// max lat
    const Y_MAX: f64 = 90.0;
    /// Max Recursion constant to use.
    const MAX_RECURSION: usize = 32;
}

impl SpaceFillingCurve2D for ZCurve2D {
    fn index(&self, x: f64, y: f64) -> i64 {
        let col = self.map_to_col(x);
        let row = self.map_to_row(y);
        Z2::new(col, row).z()
    }

    fn point(&self, index: i64) -> (f64, f64) {
        let (col, row) = Z2::new_from_zorder(index).decode();
        (self.col_to_map(col), self.row_to_map(row))
    }

    fn ranges(
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
    use crate::{Curve, SpaceFillingCurves};

    #[test]
    fn test_produce_covering_ranges() {
        let curve = SpaceFillingCurves::get_curve(Curve::ZOrder, 1024);

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
        assert_eq!(contains, true);
    }

    #[test]
    fn test_col_to_map_map_to_col() {
        let curve = ZCurve2D::new(1024);
        let m = curve.col_to_map(27);
        let col = curve.map_to_col(m);
        assert_eq!(col, 27);
    }

    #[test]
    fn point_to_index_to_point() {
        let curve = ZCurve2D::new(1024);
        let index = curve.index(-45.0, -45.0);
        let point = curve.point(index);
        assert!(point > (-45.0 - 1.0, -45.0 - 1.0));
        assert!(point < (-45.0 + 1.0, -45.0 + 1.0));
    }
}
