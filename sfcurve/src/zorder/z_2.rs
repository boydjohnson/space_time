//! A two dimensional Z-Order curve.

use crate::zorder::z_n::ZN;
use crate::zorder::z_range::ZRange;
use core::convert::TryInto;

/// A two dimensional Z-Order curve.
#[derive(Debug, PartialEq, Eq, Ord, PartialOrd)]
pub struct Z2 {
    z: i64,
}

impl Z2 {
    /// Constructor for `Z2` from values from dimension-1 and dimension-2.
    pub fn new(x: i32, y: i32) -> Self {
        Self::new_from_zorder(Self::split(x.into()) | Self::split(y.into()) << 1)
    }

    /// Create a Z2 directly from the z value.
    pub fn new_from_zorder(zorder: i64) -> Self {
        Z2 { z: zorder }
    }

    /// Index value.
    pub fn z(&self) -> i64 {
        self.z
    }

    /// Return the user space (un-z-order indexed) values.
    pub fn decode(&self) -> (i32, i32) {
        (self.dim(0), self.dim(1))
    }

    fn dim(&self, i: i32) -> i32 {
        Z2::combine(self.z >> i)
    }

    fn d0(&self) -> i32 {
        self.dim(0)
    }

    fn d1(&self) -> i32 {
        self.dim(1)
    }

    fn partial_overlaps(a1: i32, a2: i32, b1: i32, b2: i32) -> bool {
        a1.max(b1) <= a2.min(b2)
    }
}

impl ZN for Z2 {
    const DIMENSIONS: i32 = 2;

    const BITS_PER_DIMENSION: i32 = 31;

    const TOTAL_BITS: i32 = Self::DIMENSIONS * Self::BITS_PER_DIMENSION;

    const MAX_MASK: i64 = 0x7fff_ffff;

    fn split(value: i64) -> i64 {
        let mut x = value & Self::MAX_MASK;
        x = (x ^ (x << 32)) & 0x0000_0000_ffff_ffff;
        x = (x ^ (x << 16)) & 0x0000_ffff_0000_ffff;
        x = (x ^ (x << 8)) & 0x00ff_00ff_00ff_00ff;
        x = (x ^ (x << 4)) & 0x0f0f_0f0f_0f0f_0f0f;
        x = (x ^ (x << 2)) & 0x3333_3333_3333_3333;
        x = (x ^ (x << 1)) & 0x5555_5555_5555_5555;
        x
    }

    fn combine(z: i64) -> i32 {
        let mut x = z & 0x5555_5555_5555_5555;
        x = (x ^ (x >> 1)) & 0x3333_3333_3333_3333;
        x = (x ^ (x >> 2)) & 0x0f0f_0f0f_0f0f_0f0f;
        x = (x ^ (x >> 4)) & 0x00ff_00ff_00ff_00ff;
        x = (x ^ (x >> 8)) & 0x0000_ffff_0000_ffff;
        x = (x ^ (x >> 16)) & 0x0000_0000_ffff_ffff;
        x.try_into()
            .expect("combine reduces the number of bits by half.")
    }

    fn contains(range: ZRange, value: i64) -> bool {
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
    fn test_userspace_to_z2_and_back(x: u16, y: u16) -> bool {
        let (x_, y_) = Z2::new(x.into(), y.into()).decode();
        x_ == x.into() && y_ == y.into()
    }

    #[quickcheck]
    fn test_split_and_combine(x: u16) -> bool {
        Z2::combine(Z2::split(x.into())) == x.into()
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
        assert_eq!(Z2::new(i32::max_value(), 0).decode(), (i32::max_value(), 0));
        assert_eq!(Z2::new(0, i32::max_value()).decode(), (0, i32::max_value()));
        assert_eq!(
            Z2::new(i32::max_value(), i32::max_value()).decode(),
            (i32::max_value(), i32::max_value())
        );
        assert_eq!(
            Z2::new(i32::max_value() - 10, i32::max_value() - 10).decode(),
            (i32::max_value() - 10, i32::max_value() - 10)
        );
    }

    #[test]
    fn test_longest_common_prefix() {
        assert_eq!(
            Z2::longest_common_prefix(&[i64::max_value(), i64::max_value() - 15]).prefix,
            i64::max_value() - 15
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
