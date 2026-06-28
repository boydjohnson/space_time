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

use crate::{
    RangeComputeHints,
    index_range::IndexRange,
    zorder::{z_n::ZN, z_range::ZRange},
};
use alloc::{boxed::Box, vec::Vec};
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
        x = (x ^ (x >> 32)) & Self::MAX_MASK;
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
            x_max,
            y_min,
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

        let max_recurse = hints
            .iter()
            .map(|h| {
                let RangeComputeHints::MaxRecurse(max) = *h;
                if max > MAX_RECURSION {
                    MAX_RECURSION
                } else {
                    max
                }
            })
            .next();

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

#[cfg(kani)]
mod kani_proofs {
    use super::*;

    /// Splitting then combining a value within `MAX_MASK` is the identity, for
    /// every valid input. Also proves the `try_into().expect(..)` in `combine`
    /// never panics.
    #[kani::proof]
    fn split_combine_roundtrip() {
        let x: u32 = kani::any();
        kani::assume(x <= Z3::MAX_MASK as u32);
        assert_eq!(Z3::combine(Z3::split(x)), x);
    }

    /// Encoding three dimensions into a `Z3` and decoding recovers the original
    /// triple, for every valid input.
    #[kani::proof]
    fn encode_decode_roundtrip() {
        let x: u32 = kani::any();
        let y: u32 = kani::any();
        let z: u32 = kani::any();
        kani::assume(x <= Z3::MAX_MASK as u32);
        kani::assume(y <= Z3::MAX_MASK as u32);
        kani::assume(z <= Z3::MAX_MASK as u32);
        assert_eq!(Z3::new(x, y, z).decode(), (x, y, z));
    }

    /// Build a `ZRange` bounding box from two ordered user-space corners.
    fn box_from(x0: u32, y0: u32, z0: u32, x1: u32, y1: u32, z1: u32) -> ZRange {
        ZRange {
            min: Z3::new(x0, y0, z0).z,
            max: Z3::new(x1, y1, z1).z,
        }
    }

    /// Any user-space point inside a bounding box (built from two ordered
    /// corners) is reported as contained by `Z3::contains`. This proves the
    /// user-space containment semantics of the index-space predicate.
    #[kani::proof]
    fn point_in_box_is_contained() {
        let x0: u32 = kani::any();
        let y0: u32 = kani::any();
        let z0: u32 = kani::any();
        let x1: u32 = kani::any();
        let y1: u32 = kani::any();
        let z1: u32 = kani::any();
        kani::assume(x0 <= x1 && x1 <= Z3::MAX_MASK as u32);
        kani::assume(y0 <= y1 && y1 <= Z3::MAX_MASK as u32);
        kani::assume(z0 <= z1 && z1 <= Z3::MAX_MASK as u32);

        let px: u32 = kani::any();
        let py: u32 = kani::any();
        let pz: u32 = kani::any();
        kani::assume(x0 <= px && px <= x1);
        kani::assume(y0 <= py && py <= y1);
        kani::assume(z0 <= pz && pz <= z1);

        let range = box_from(x0, y0, z0, x1, y1, z1);
        assert!(Z3::contains(range, Z3::new(px, py, pz).z));
    }

    /// For bounding boxes built from ordered corners, if `range` contains both
    /// corners of `value` then `range` and `value` overlap. A containing range
    /// must overlap.
    #[kani::proof]
    fn contains_value_implies_overlaps() {
        let rx0: u32 = kani::any();
        let ry0: u32 = kani::any();
        let rz0: u32 = kani::any();
        let rx1: u32 = kani::any();
        let ry1: u32 = kani::any();
        let rz1: u32 = kani::any();
        kani::assume(rx0 <= rx1 && rx1 <= Z3::MAX_MASK as u32);
        kani::assume(ry0 <= ry1 && ry1 <= Z3::MAX_MASK as u32);
        kani::assume(rz0 <= rz1 && rz1 <= Z3::MAX_MASK as u32);

        let vx0: u32 = kani::any();
        let vy0: u32 = kani::any();
        let vz0: u32 = kani::any();
        let vx1: u32 = kani::any();
        let vy1: u32 = kani::any();
        let vz1: u32 = kani::any();
        kani::assume(vx0 <= vx1 && vx1 <= Z3::MAX_MASK as u32);
        kani::assume(vy0 <= vy1 && vy1 <= Z3::MAX_MASK as u32);
        kani::assume(vz0 <= vz1 && vz1 <= Z3::MAX_MASK as u32);

        let range = box_from(rx0, ry0, rz0, rx1, ry1, rz1);
        let value = box_from(vx0, vy0, vz0, vx1, vy1, vz1);

        kani::assume(Z3::contains_value(range, value));
        assert!(Z3::overlaps(range, value));
    }

    /// `Z3::overlaps` is symmetric for every pair of index-space rectangles.
    #[kani::proof]
    fn overlaps_is_symmetric() {
        let a_min: u64 = kani::any();
        let a_max: u64 = kani::any();
        let b_min: u64 = kani::any();
        let b_max: u64 = kani::any();
        let a = ZRange {
            min: a_min,
            max: a_max,
        };
        let b = ZRange {
            min: b_min,
            max: b_max,
        };
        assert_eq!(Z3::overlaps(a, b), Z3::overlaps(b, a));
    }

    /// A point is contained by a bounding box exactly when the box overlaps the
    /// degenerate range made of just that point. Ties `contains` and `overlaps`
    /// together.
    #[kani::proof]
    fn contains_matches_degenerate_overlap() {
        let x0: u32 = kani::any();
        let y0: u32 = kani::any();
        let z0: u32 = kani::any();
        let x1: u32 = kani::any();
        let y1: u32 = kani::any();
        let z1: u32 = kani::any();
        kani::assume(x0 <= x1 && x1 <= Z3::MAX_MASK as u32);
        kani::assume(y0 <= y1 && y1 <= Z3::MAX_MASK as u32);
        kani::assume(z0 <= z1 && z1 <= Z3::MAX_MASK as u32);

        let px: u32 = kani::any();
        let py: u32 = kani::any();
        let pz: u32 = kani::any();
        kani::assume(px <= Z3::MAX_MASK as u32);
        kani::assume(py <= Z3::MAX_MASK as u32);
        kani::assume(pz <= Z3::MAX_MASK as u32);

        let range = box_from(x0, y0, z0, x1, y1, z1);
        let point = Z3::new(px, py, pz).z;
        let degenerate = ZRange {
            min: point,
            max: point,
        };

        assert_eq!(Z3::contains(range, point), Z3::overlaps(range, degenerate));
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use quickcheck_macros::quickcheck;

    /// Space-time point-curve analogue of the XZ `index_in_ranges_when_overlaps`
    /// contract: a point is stored under `index(x, y, t)`, and a box+time query
    /// returns `ranges(...)`. COMPLETENESS — if the point's grid cell falls
    /// inside the query's (col, row, depth) rectangle, then `index(point)` must
    /// land in one of the returned ranges, for ANY recursion depth (`MaxRecurse`
    /// is the z-order analogue of XZ's `max_ranges` cap). Ground truth is taken
    /// in the curve's own grid so the comparison is exact.
    #[quickcheck]
    fn zcurve3d_index_in_ranges_when_point_in_box(
        g_seed: u8,
        point: (u16, u16, u16),
        bbox: (u16, u16, u16, u16),
        time: (u16, u16),
        recurse: u8,
    ) -> quickcheck::TestResult {
        let g = 1u32 << (u32::from(g_seed % 10) + 1); // 2..=1024
        let z_max = 1000.0;
        let curve = ZCurve3D::new(g, -180.0, -90.0, 180.0, 90.0, z_max);

        let to_x = |v: u16| -180.0 + f64::from(v) / f64::from(u16::MAX) * 360.0;
        let to_y = |v: u16| -90.0 + f64::from(v) / f64::from(u16::MAX) * 180.0;
        let to_t = |v: u16| f64::from(v) / f64::from(u16::MAX) * z_max;

        let (px, py, pt) = (to_x(point.0), to_y(point.1), to_t(point.2));
        let (mut qxmin, mut qxmax) = (to_x(bbox.0), to_x(bbox.2));
        if qxmin > qxmax {
            core::mem::swap(&mut qxmin, &mut qxmax);
        }
        let (mut qymin, mut qymax) = (to_y(bbox.1), to_y(bbox.3));
        if qymin > qymax {
            core::mem::swap(&mut qymin, &mut qymax);
        }
        let (mut qtmin, mut qtmax) = (to_t(time.0), to_t(time.1));
        if qtmin > qtmax {
            core::mem::swap(&mut qtmin, &mut qtmax);
        }

        // Ground truth in grid space, mirroring how `ranges` builds its corners.
        let pcol = curve.map_to_col(px);
        let prow = curve.map_to_row(py);
        let pdepth = curve.time_to_depth(pt);
        let in_box = pcol >= curve.map_to_col(qxmin)
            && pcol <= curve.map_to_col(qxmax)
            && prow >= curve.map_to_row(qymax)
            && prow <= curve.map_to_row(qymin)
            && pdepth >= curve.time_to_depth(qtmin)
            && pdepth <= curve.time_to_depth(qtmax);
        if !in_box {
            return quickcheck::TestResult::discard();
        }

        let index = curve.index(px, py, pt);
        let hints = if recurse == 0 {
            Vec::new()
        } else {
            alloc::vec![RangeComputeHints::MaxRecurse(usize::from(recurse))]
        };
        let ranges = curve.ranges(qxmin, qymin, qxmax, qymax, qtmin, qtmax, &hints);

        let covered = ranges
            .iter()
            .any(|r| r.lower() <= index && index <= r.upper());
        quickcheck::TestResult::from_bool(covered)
    }

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
            Z3::new(u16::MAX as u32, 0, 0).decode(),
            (u16::MAX as u32, 0, 0)
        );
        assert_eq!(
            Z3::new(u16::MAX as u32, 0, u16::MAX as u32).decode(),
            (u16::MAX as u32, 0, u16::MAX as u32)
        );
    }

    #[quickcheck]
    fn test_encode_decode(x: u16, y: u16, z: u16) -> bool {
        Z3::new(x.into(), y.into(), z.into()).decode() == (x.into(), y.into(), z.into())
    }

    #[quickcheck]
    fn test_encode_decode_full_range(x: u32, y: u32, z: u32) -> bool {
        // Exercise the full 21-bit `MAX_MASK` range, not just the lower 16 bits.
        let x = x & Z3::MAX_MASK as u32;
        let y = y & Z3::MAX_MASK as u32;
        let z = z & Z3::MAX_MASK as u32;
        Z3::new(x, y, z).decode() == (x, y, z)
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

        assert!(
            minneapolis_1995_query
                .iter()
                .any(|r| r.lower() <= minneapolis_1995 && r.upper() >= minneapolis_1995)
        );
        assert!(
            !minneapolis_1995_query
                .iter()
                .any(|r| r.lower() <= minneapolis_2005 && r.upper() >= minneapolis_2005)
        );
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
                    assert!(
                        range
                            .iter()
                            .any(|r| r.lower() <= indexed_point && indexed_point <= r.upper())
                    );

                    t += 10_000_000.0;
                }

                lat += 5.0;
            }
            lon += 5.0;
        }
    }
}

#[cfg(test)]
mod range_query_tests {
    use super::*;
    use quickcheck_macros::quickcheck;

    /// End-to-end property for the range engine over `Z3`: for any rectangular
    /// (3-D) query box on a small grid, and any bottom-out settings, `zranges`
    /// must be
    ///
    ///  * COMPLETE — every cell inside the box falls in some returned range, so a query
    ///    never silently drops results, and
    ///  * SOUND — every index inside a `CoveredRange` decodes back into the box
    ///    (overlapping ranges are allowed to spill, covered ones are not).
    ///
    /// `max_recurse`/`max_ranges` are varied (including the aggressive values
    /// that force early `bottom_out`) to confirm completeness survives the
    /// coarsening.
    #[quickcheck]
    fn z3_zranges_complete_and_sound(
        bits: u8,
        coords: (u8, u8, u8, u8, u8, u8),
        max_recurse: Option<u8>,
        max_ranges: Option<u8>,
    ) -> bool {
        let (x0, y0, z0, x1, y1, z1) = coords;

        // Grid side 2..=8 (powers of two; box volume stays <= 512 cells).
        let n_bits: u32 = u32::from(bits % 3) + 1;
        let side: u32 = 1 << n_bits;

        let (col_min, col_max) = {
            let (lo, hi) = (u32::from(x0) % side, u32::from(x1) % side);
            (lo.min(hi), lo.max(hi))
        };
        let (row_min, row_max) = {
            let (lo, hi) = (u32::from(y0) % side, u32::from(y1) % side);
            (lo.min(hi), lo.max(hi))
        };
        let (dep_min, dep_max) = {
            let (lo, hi) = (u32::from(z0) % side, u32::from(z1) % side);
            (lo.min(hi), lo.max(hi))
        };

        let zbound = ZRange {
            min: Z3::new(col_min, row_min, dep_min).z,
            max: Z3::new(col_max, row_max, dep_max).z,
        };

        let max_recurse = max_recurse.map(|v| usize::from(v % 10)); // 0..=9
        let max_ranges = max_ranges.map(|v| usize::from(v % 32) + 1); // 1..=32

        // 64 is the precision the public `ranges()` entry points always pass.
        let ranges = <Z3 as ZN>::zranges::<Z3>(&[zbound], 64, max_ranges, max_recurse);

        let cell_count = u64::from(side) * u64::from(side) * u64::from(side);

        // Completeness.
        for col in col_min..=col_max {
            for row in row_min..=row_max {
                for dep in dep_min..=dep_max {
                    let z = Z3::new(col, row, dep).z;
                    if !ranges.iter().any(|rg| rg.lower() <= z && z <= rg.upper()) {
                        return false;
                    }
                }
            }
        }

        // Soundness of covered ranges.
        for rg in &ranges {
            if rg.contained() {
                // A sound covered range can hold at most `box_cells` (< grid
                // cells) consecutive in-box indices; a larger span is unsound on
                // its face and also guards against runaway enumeration on a bug.
                if rg.upper().saturating_sub(rg.lower()) >= cell_count {
                    return false;
                }
                for z in rg.lower()..=rg.upper() {
                    let (col, row, dep) = Z3::new_from_raw(z).decode();
                    if col < col_min
                        || col > col_max
                        || row < row_min
                        || row > row_max
                        || dep < dep_min
                        || dep > dep_max
                    {
                        return false;
                    }
                }
            }
        }

        true
    }
}
