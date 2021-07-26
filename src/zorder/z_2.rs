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
        Z2::combine(Z2::split(x)) == x
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
            Z2::longest_common_prefix(&[u64::max_value(), u64::max_value() - 15]).prefix,
            u64::max_value() - 15
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
