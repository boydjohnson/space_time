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

//! A two dimensional Z-Order curve.

use crate::zorder::{z_n::ZN, z_range::ZRange};
use core::convert::TryInto;

/// A two dimensional Z-Order curve.
#[derive(Debug, PartialEq, Eq, Ord, PartialOrd)]
pub struct Z2 {
    z: u64,
}

impl Z2 {
    /// Constructor for `Z2` from values from dimension-1 and dimension-2.
    #[must_use]
    pub fn new(x: u32, y: u32) -> Self {
        assert!(x <= Self::MAX_MASK as u32);
        assert!(y <= Self::MAX_MASK as u32);

        Self::new_from_zorder(Self::split(x) | Self::split(y) << 1)
    }

    /// Create a Z2 directly from the z value.
    #[must_use]
    pub fn new_from_zorder(zorder: u64) -> Self {
        Z2 { z: zorder }
    }

    /// Index value.
    #[must_use]
    pub fn z(&self) -> u64 {
        self.z
    }

    /// Return the user space (un-z-order indexed) values.
    #[must_use]
    pub fn decode(&self) -> (u32, u32) {
        (self.dim(0), self.dim(1))
    }

    fn dim(&self, i: u64) -> u32 {
        Z2::combine(self.z >> i)
    }

    fn d0(&self) -> u32 {
        self.dim(0)
    }

    fn d1(&self) -> u32 {
        self.dim(1)
    }

    fn partial_overlaps(a1: u32, a2: u32, b1: u32, b2: u32) -> bool {
        a1.max(b1) <= a2.min(b2)
    }
}

impl ZN for Z2 {
    const DIMENSIONS: u64 = 2;

    const BITS_PER_DIMENSION: u32 = 31;

    const TOTAL_BITS: u64 = Self::DIMENSIONS * Self::BITS_PER_DIMENSION as u64;

    const MAX_MASK: u64 = 0x7fff_ffff;

    fn split(value: u32) -> u64 {
        let mut x = value.into();
        x &= Self::MAX_MASK;
        x = (x | (x << 32)) & 0x0000_0000_ffff_ffff_u64;
        x = (x | (x << 16)) & 0x0000_ffff_0000_ffff_u64;
        x = (x | (x << 8)) & 0x00ff_00ff_00ff_00ff_u64;
        x = (x | (x << 4)) & 0x0f0f_0f0f_0f0f_0f0f_u64;
        x = (x | (x << 2)) & 0x3333_3333_3333_3333_u64;
        x = (x | (x << 1)) & 0x5555_5555_5555_5555_u64;
        x
    }

    fn combine(z: u64) -> u32 {
        let mut x = z & 0x5555_5555_5555_5555;
        x = (x ^ (x >> 1)) & 0x3333_3333_3333_3333;
        x = (x ^ (x >> 2)) & 0x0f0f_0f0f_0f0f_0f0f;
        x = (x ^ (x >> 4)) & 0x00ff_00ff_00ff_00ff;
        x = (x ^ (x >> 8)) & 0x0000_ffff_0000_ffff;
        x = (x ^ (x >> 16)) & 0x0000_0000_ffff_ffff;
        x.try_into().expect("Value fits into a u32")
    }

    fn contains(range: ZRange, value: u64) -> bool {
        let (x, y) = Z2 { z: value }.decode();
        x >= Z2 { z: range.min }.d0()
            && x <= Z2 { z: range.max }.d0()
            && y >= Z2 { z: range.min }.d1()
            && y <= Z2 { z: range.max }.d1()
    }

    fn overlaps(range: ZRange, value: ZRange) -> bool {
        Self::partial_overlaps(
            Z2 { z: range.min }.d0(),
            Z2 { z: range.max }.d0(),
            Z2 { z: value.min }.d0(),
            Z2 { z: value.max }.d0(),
        ) && Self::partial_overlaps(
            Z2 { z: range.min }.d1(),
            Z2 { z: range.max }.d1(),
            Z2 { z: value.min }.d1(),
            Z2 { z: value.max }.d1(),
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
        kani::assume(x <= Z2::MAX_MASK as u32);
        assert_eq!(Z2::combine(Z2::split(x)), x);
    }

    /// Encoding two dimensions into a `Z2` and decoding recovers the original
    /// pair, for every valid input.
    #[kani::proof]
    fn encode_decode_roundtrip() {
        let x: u32 = kani::any();
        let y: u32 = kani::any();
        kani::assume(x <= Z2::MAX_MASK as u32);
        kani::assume(y <= Z2::MAX_MASK as u32);
        assert_eq!(Z2::new(x, y).decode(), (x, y));
    }

    /// Build a `ZRange` bounding box from two ordered user-space corners.
    fn box_from(x0: u32, y0: u32, x1: u32, y1: u32) -> ZRange {
        ZRange {
            min: Z2::new(x0, y0).z(),
            max: Z2::new(x1, y1).z(),
        }
    }

    /// Any user-space point inside a bounding box (built from two ordered
    /// corners) is reported as contained by `Z2::contains`. This proves the
    /// user-space containment semantics of the index-space predicate.
    #[kani::proof]
    fn point_in_box_is_contained() {
        let x0: u32 = kani::any();
        let y0: u32 = kani::any();
        let x1: u32 = kani::any();
        let y1: u32 = kani::any();
        kani::assume(x0 <= x1 && x1 <= Z2::MAX_MASK as u32);
        kani::assume(y0 <= y1 && y1 <= Z2::MAX_MASK as u32);

        let px: u32 = kani::any();
        let py: u32 = kani::any();
        kani::assume(x0 <= px && px <= x1);
        kani::assume(y0 <= py && py <= y1);

        let range = box_from(x0, y0, x1, y1);
        assert!(Z2::contains(range, Z2::new(px, py).z()));
    }

    /// For bounding boxes built from ordered corners, if `range` contains both
    /// corners of `value` then `range` and `value` overlap. A containing range
    /// must overlap.
    #[kani::proof]
    fn contains_value_implies_overlaps() {
        let rx0: u32 = kani::any();
        let ry0: u32 = kani::any();
        let rx1: u32 = kani::any();
        let ry1: u32 = kani::any();
        kani::assume(rx0 <= rx1 && rx1 <= Z2::MAX_MASK as u32);
        kani::assume(ry0 <= ry1 && ry1 <= Z2::MAX_MASK as u32);

        let vx0: u32 = kani::any();
        let vy0: u32 = kani::any();
        let vx1: u32 = kani::any();
        let vy1: u32 = kani::any();
        kani::assume(vx0 <= vx1 && vx1 <= Z2::MAX_MASK as u32);
        kani::assume(vy0 <= vy1 && vy1 <= Z2::MAX_MASK as u32);

        let range = box_from(rx0, ry0, rx1, ry1);
        let value = box_from(vx0, vy0, vx1, vy1);

        kani::assume(Z2::contains_value(range, value));
        assert!(Z2::overlaps(range, value));
    }

    /// `Z2::overlaps` is symmetric for every pair of index-space rectangles.
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
        assert_eq!(Z2::overlaps(a, b), Z2::overlaps(b, a));
    }

    /// A point is contained by a bounding box exactly when the box overlaps the
    /// degenerate range made of just that point. Ties `contains` and `overlaps`
    /// together.
    #[kani::proof]
    fn contains_matches_degenerate_overlap() {
        let x0: u32 = kani::any();
        let y0: u32 = kani::any();
        let x1: u32 = kani::any();
        let y1: u32 = kani::any();
        kani::assume(x0 <= x1 && x1 <= Z2::MAX_MASK as u32);
        kani::assume(y0 <= y1 && y1 <= Z2::MAX_MASK as u32);

        let px: u32 = kani::any();
        let py: u32 = kani::any();
        kani::assume(px <= Z2::MAX_MASK as u32);
        kani::assume(py <= Z2::MAX_MASK as u32);

        let range = box_from(x0, y0, x1, y1);
        let point = Z2::new(px, py).z();
        let degenerate = ZRange {
            min: point,
            max: point,
        };

        assert_eq!(Z2::contains(range, point), Z2::overlaps(range, degenerate));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[quickcheck]
    fn test_userspace_to_z2_and_back(x: u32, y: u32) -> bool {
        if x > Z2::MAX_MASK as u32 || y > Z2::MAX_MASK as u32 {
            true
        } else {
            let (x_, y_) = Z2::new(x, y).decode();
            x_ == x && y_ == y
        }
    }

    #[quickcheck]
    fn test_split_and_combine(x: u32) -> bool {
        if x > Z2::MAX_MASK as u32 {
            true
        } else {
            Z2::combine(Z2::split(x)) == x
        }
    }

    #[test]
    fn test_z2_encoding() {
        assert_eq!(Z2::new(1, 0).z, 1);
        assert_eq!(Z2::new(2, 0).z, 4);
        assert_eq!(Z2::new(3, 0).z, 5);
        assert_eq!(Z2::new(4, 0).z, 16);
        assert_eq!(Z2::new(0, 1).z, 2);
        assert_eq!(Z2::new(0, 2).z, 8);
        assert_eq!(Z2::new(0, 3).z, 10);
    }

    #[test]
    fn test_z2_decoding() {
        assert_eq!(Z2::new(23, 13).decode(), (23, 13));
        assert_eq!(
            Z2::new(Z2::MAX_MASK as u32, 0).decode(),
            (Z2::MAX_MASK as u32, 0)
        );
        assert_eq!(
            Z2::new(0, Z2::MAX_MASK as u32).decode(),
            (0, Z2::MAX_MASK as u32)
        );
        assert_eq!(
            Z2::new(Z2::MAX_MASK as u32, Z2::MAX_MASK as u32).decode(),
            (Z2::MAX_MASK as u32, Z2::MAX_MASK as u32)
        );
        assert_eq!(
            Z2::new(Z2::MAX_MASK as u32 - 10, Z2::MAX_MASK as u32 - 10).decode(),
            (Z2::MAX_MASK as u32 - 10, Z2::MAX_MASK as u32 - 10)
        );
    }

    #[test]
    fn test_longest_common_prefix() {
        assert_eq!(
            Z2::longest_common_prefix(&[u64::MAX, u64::MAX - 15]).prefix,
            u64::MAX - 15
        );
        assert_eq!(Z2::longest_common_prefix(&[15, 13]).prefix, 12); // 1111, 1101 => 1100 => 12
        assert_eq!(Z2::longest_common_prefix(&[12, 15]).prefix, 12); // 1100, 1111 => 1100
                                                                     // => 12
    }

    #[test]
    fn test_zrange() {
        let ranges = Z2::zranges_default::<Z2>(&[ZRange { min: 12, max: 15 }]);

        assert_eq!(ranges.len(), 1);
        assert_eq!(ranges[0].lower(), 12);
        assert_eq!(ranges[0].upper(), 15);

        let ranges = Z2::zranges_default::<Z2>(&[ZRange { min: 0, max: 15 }]);
        assert_eq!(ranges.len(), 1);
        assert_eq!(ranges[0].lower(), 0);
        assert_eq!(ranges[0].upper(), 15);

        let ranges = Z2::zranges_default::<Z2>(&[ZRange { min: 0, max: 27 }]);
        assert_eq!(ranges.len(), 2);
        assert_eq!(ranges[0].lower(), 0);
        assert_eq!(ranges[0].upper(), 19);
        assert_eq!(ranges[1].lower(), 24);
        assert_eq!(ranges[1].upper(), 27);
    }

    #[test]
    fn test_contains() {
        let z_range_1 = ZRange { min: 0, max: 3 };
        let z_range_2 = ZRange { min: 2, max: 3 };
        assert!(Z2::contains_value(z_range_1, z_range_2));

        assert!(Z2::contains(ZRange { min: 2, max: 6 }, 3));
    }

    #[test]
    fn test_overlaps() {
        assert!(Z2::overlaps(
            ZRange { min: 0, max: 1 },
            ZRange { min: 1, max: 4 }
        ));
        // Smaller overlaps larger
        assert!(Z2::overlaps(
            ZRange {
                min: Z2::new(1, 0).z(),
                max: Z2::new(2, 0).z()
            },
            ZRange {
                min: Z2::new(0, 0).z(),
                max: Z2::new(4, 0).z()
            }
        ));
        // larger overlaps smaller
        assert!(Z2::overlaps(
            ZRange {
                min: Z2::new(0, 0).z(),
                max: Z2::new(4, 0).z()
            },
            ZRange {
                min: Z2::new(1, 0).z(),
                max: Z2::new(2, 0).z()
            }
        ));
    }
}

#[cfg(test)]
mod range_query_tests {
    use super::*;
    use quickcheck_macros::quickcheck;

    /// End-to-end property for the range engine over `Z2`: for any rectangular
    /// query box on a small grid, and any bottom-out settings, `zranges` must be
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
    fn z2_zranges_complete_and_sound(
        bits: u8,
        a: u16,
        b: u16,
        c: u16,
        d: u16,
        max_recurse: Option<u8>,
        max_ranges: Option<u8>,
    ) -> bool {
        // Grid side 2..=32 (powers of two so every z in `0..side*side` is a cell).
        let n_bits: u32 = u32::from(bits % 5) + 1;
        let side: u32 = 1 << n_bits;

        let (col_min, col_max) = {
            let (lo, hi) = (u32::from(a) % side, u32::from(c) % side);
            (lo.min(hi), lo.max(hi))
        };
        let (row_min, row_max) = {
            let (lo, hi) = (u32::from(b) % side, u32::from(d) % side);
            (lo.min(hi), lo.max(hi))
        };

        let zbound = ZRange {
            min: Z2::new(col_min, row_min).z(),
            max: Z2::new(col_max, row_max).z(),
        };

        let max_recurse = max_recurse.map(|v| usize::from(v % 10)); // 0..=9
        let max_ranges = max_ranges.map(|v| usize::from(v % 32) + 1); // 1..=32

        // 64 is the precision the public `ranges()` entry points always pass.
        let ranges = Z2::zranges::<Z2>(&[zbound], 64, max_ranges, max_recurse);

        let cell_count = u64::from(side) * u64::from(side);

        // Completeness.
        for col in col_min..=col_max {
            for row in row_min..=row_max {
                let z = Z2::new(col, row).z();
                if !ranges.iter().any(|rg| rg.lower() <= z && z <= rg.upper()) {
                    return false;
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
                    let (col, row) = Z2::new_from_zorder(z).decode();
                    if col < col_min || col > col_max || row < row_min || row > row_max {
                        return false;
                    }
                }
            }
        }

        true
    }
}
