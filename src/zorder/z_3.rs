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

//! A three dimensional space filling curve.

use crate::index_range::IndexRange;
use crate::zorder::z_n::ZN;
use crate::zorder::z_range::ZRange;
use crate::RangeComputeHints;
use alloc::boxed::Box;
use alloc::vec::Vec;
use core::convert::TryInto;

/// Three dimensional space filling curve.
pub struct Z3 {
    z: u64,
}

impl Z3 {
    /// New Z3 from z-index value.
    #[must_use]
    pub fn new_from_raw(z: u64) -> Self {
        Z3 { z }
    }

    fn d0(&self) -> u32 {
        Self::combine(self.z)
    }

    fn d1(&self) -> u32 {
        Self::combine(self.z >> 1)
    }

    fn d2(&self) -> u32 {
        Self::combine(self.z >> 2)
    }

    fn decode(&self) -> (u32, u32, u32) {
        (self.d0(), self.d1(), self.d2())
    }

    /// Constructor.
    #[must_use]
    pub fn new(x: u32, y: u32, z: u32) -> Self {
        assert!(x <= Self::MAX_MASK as u32);
        assert!(y <= Self::MAX_MASK as u32);
        assert!(z <= Self::MAX_MASK as u32);

        Z3 {
            z: Self::split(x) | Self::split(y) << 1 | Self::split(z) << 2,
        }
    }

    fn partial_overlaps(a1: u32, a2: u32, b1: u32, b2: u32) -> bool {
        a1.max(b1) <= a2.min(b2)
    }
}

impl ZN for Z3 {
    const DIMENSIONS: u64 = 3;
    const BITS_PER_DIMENSION: u32 = 21;
    const TOTAL_BITS: u64 = 63;
    const MAX_MASK: u64 = 0x1f_ffff;

    fn split(value: u32) -> u64 {
        let mut x: u64 = value.into();
        x &= Self::MAX_MASK;
        x = (x | x << 32) & 0x1f_0000_0000_ffff_u64;
        x = (x | x << 16) & 0x1f_0000_ff00_00ff_u64;
        x = (x | x << 8) & 0x100f_00f0_0f00_f00f_u64;
        x = (x | x << 4) & 0x10c3_0c30_c30c_30c3_u64;
        x = (x | x << 2) & 0x1249_2492_4924_9249_u64;
        x
    }

    fn combine(z: u64) -> u32 {
        let mut x = z & 0x1249_2492_4924_9249;
        x = (x ^ (x >> 2)) & 0x10c3_0c30_c30c_30c3;
        x = (x ^ (x >> 4)) & 0x100f_00f0_0f00_f00f;
        x = (x ^ (x >> 8)) & 0x1f_0000_ff00_00ff;
        x = (x ^ (x >> 16)) & 0x1f_0000_0000_ffff;
        x = x ^ (x >> 32);
        x.try_into()
            .expect("values were chosen so x fits into a u32")
    }

    fn contains(range: ZRange, value: u64) -> bool {
        let (x, y, z) = Z3::new_from_raw(value).decode();
        x >= Z3 { z: range.min }.d0()
            && x <= Z3 { z: range.max }.d0()
            && y >= Z3 { z: range.min }.d1()
            && y <= Z3 { z: range.max }.d1()
            && z >= Z3 { z: range.min }.d2()
            && z <= Z3 { z: range.max }.d2()
    }

    fn overlaps(range: ZRange, value: ZRange) -> bool {
        let range_min = Z3 { z: range.min };
        let range_max = Z3 { z: range.max };
        let value_min = Z3 { z: value.min };
        let value_max = Z3 { z: value.max };

        Self::partial_overlaps(
            range_min.d0(),
            range_max.d0(),
            value_min.d0(),
            value_max.d0(),
        ) && Self::partial_overlaps(
            range_min.d1(),
            range_max.d1(),
            value_min.d1(),
            value_max.d1(),
        ) && Self::partial_overlaps(
            range_min.d2(),
            range_max.d2(),
            value_min.d2(),
            value_max.d2(),
        )
    }
}

/// A nice interface into a curve to index a point and time.
pub struct ZCurve3D {
    g: u32,
    x_min: f64,
    x_max: f64,
    y_min: f64,
    y_max: f64,
    z_max: f64,
}

const MAX_RECURSION: usize = 32;

impl Default for ZCurve3D {
    fn default() -> ZCurve3D {
        ZCurve3D::new(10_000, -180.0, -90.0, 180.0, 90.0, 2_556_057_600.0)
    }
}

impl ZCurve3D {
    /// Constructor with bounds on the space-time that this index will act on.
    #[must_use]
    pub fn new(g: u32, x_min: f64, y_min: f64, x_max: f64, y_max: f64, z_max: f64) -> Self {
        ZCurve3D {
            g,
            x_min,
            y_min,
            x_max,
            y_max,
            z_max,
        }
    }

    fn cell_height(&self) -> f64 {
        (self.y_max - self.y_min) / f64::from(self.g)
    }

    fn cell_width(&self) -> f64 {
        (self.x_max - self.x_min) / f64::from(self.g)
    }

    fn cell_depth(&self) -> f64 {
        self.z_max / f64::from(self.g)
    }

    fn map_to_col(&self, x: f64) -> u32 {
        ((x - self.x_min) / self.cell_width()) as u32
    }

    fn map_to_row(&self, y: f64) -> u32 {
        ((self.y_max - y) / self.cell_height()) as u32
    }

    fn time_to_depth(&self, z: f64) -> u32 {
        (z / self.cell_depth()) as u32
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

    fn depth_to_time(&self, depth: u32) -> f64 {
        (f64::from(depth) * self.cell_depth() + self.cell_height() / 2.0)
            .min(self.z_max)
            .max(0.0)
    }

    /// Index a `x` longitude, `y` latitude, and a timestamp `t`.
    #[must_use]
    pub fn index(&self, x: f64, y: f64, t: f64) -> u64 {
        Z3::new(
            self.map_to_col(x),
            self.map_to_row(y),
            self.time_to_depth(t),
        )
        .z
    }

    /// Return the x,y,t from an index.
    #[must_use]
    pub fn invert(&self, i: u64) -> (f64, f64, f64) {
        let (col, row, depth) = Z3::new_from_raw(i).decode();
        (
            self.col_to_map(col),
            self.row_to_map(row),
            self.depth_to_time(depth),
        )
    }

    /// Return the `IndexRange`s that cover the bounding box and time range.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn ranges(
        &self,
        x_min: f64,
        y_min: f64,
        x_max: f64,
        y_max: f64,
        t_min: f64,
        t_max: f64,
        hints: &[RangeComputeHints],
    ) -> Vec<Box<dyn IndexRange>> {
        let col_min = self.map_to_col(x_min);
        let row_min = self.map_to_row(y_max);
        let depth_min: u32 = self.time_to_depth(t_min);
        let min = Z3::new(col_min, row_min, depth_min);

        let col_max = self.map_to_col(x_max);
        let row_max = self.map_to_row(y_min);
        let depth_max: u32 = self.time_to_depth(t_max);
        let max = Z3::new(col_max, row_max, depth_max);

        let max_recurse = hints.iter().find_map(|h| {
            let RangeComputeHints::MaxRecurse(max) = *h;
            if max > MAX_RECURSION {
                Some(MAX_RECURSION)
            } else {
                Some(max)
            }
        });

        <Z3 as ZN>::zranges::<Z3>(
            &[ZRange {
                min: min.z,
                max: max.z,
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

    #[test]
    fn test_encode() {
        assert_eq!(Z3::new(1, 0, 0).z, 1);
        assert_eq!(Z3::new(0, 1, 0).z, 2);
        assert_eq!(Z3::new(0, 0, 1).z, 4);
        assert_eq!(Z3::new(1, 1, 1).z, 7);
    }

    #[test]
    fn test_decode() {
        assert_eq!(Z3::new(23, 13, 200).decode(), (23, 13, 200));
        // only 21 bits are saved, so MAX Value gets chopped
        assert_eq!(
            Z3::new(u16::max_value() as u32, 0, 0).decode(),
            (u16::max_value() as u32, 0, 0)
        );
        assert_eq!(
            Z3::new(u16::max_value() as u32, 0, u16::max_value() as u32).decode(),
            (u16::max_value() as u32, 0, u16::max_value() as u32)
        );
    }

    #[quickcheck]
    fn test_encode_decode(x: u16, y: u16, z: u16) -> bool {
        Z3::new(x.into(), y.into(), z.into()).decode() == (x.into(), y.into(), z.into())
    }

    #[test]
    fn test_z3_time_curve() {
        let curve = ZCurve3D::new(1024, -180.0, -90.0, 180.0, 90.0, 1207632712000.0);

        let minneapolis_1995 = curve.index(-93.2650, 44.9778, 792013512000.0); // Minneapolis, 1995.
        let minneapolis_2005 = curve.index(-93.2650, 44.9778, 1107632712000.0); //Minneapolis, 2005.

        let minneapolis_1995_query = curve.ranges(
            -93.266,
            44.9777,
            -93.264,
            44.9779,
            792013412000.0,
            792013612000.0,
            &[],
        );

        assert!(minneapolis_1995_query
            .iter()
            .any(|r| r.lower() <= minneapolis_1995 && r.upper() >= minneapolis_1995));
        assert!(!minneapolis_1995_query
            .iter()
            .any(|r| r.lower() <= minneapolis_2005 && r.upper() >= minneapolis_2005));
    }

    #[test]
    fn test_sweep_through_map() {
        let curve = ZCurve3D::default();

        let mut lon = -180.0;
        let mut lat = -90.0;
        let mut t = 0.0;

        while lon < 180.0 {
            while lat < 90.0 {
                while t < 2_556_057_600.0 {
                    let indexed_point = curve.index(lon, lat, t);
                    let range = curve.ranges(
                        (lon - 10.0).max(-180.0),
                        (lat - 10.0).max(-90.0),
                        (lon + 10.0).min(180.0),
                        (lat + 10.0).min(90.0),
                        (t - 10.0).max(0.0),
                        (t + 10.0).min(2_556_057_600.0),
                        &[],
                    );
                    assert!(range
                        .iter()
                        .any(|r| r.lower() <= indexed_point && indexed_point <= r.upper()));

                    t += 10_000_000.0;
                }

                lat += 5.0;
            }
            lon += 5.0;
        }
    }
}
