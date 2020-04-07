//! A three dimensional space filling curve.

use crate::zorder::z_n::ZN;
use crate::zorder::z_range::ZRange;
use core::convert::TryInto;

/// Three dimensional space filling curve.
pub struct Z3 {
    z: i64,
}

impl Z3 {
    /// New Z3 from z-index value.
    pub fn new_from_raw(z: i64) -> Self {
        Z3 { z }
    }

    fn d0(&self) -> i32 {
        Self::combine(self.z)
    }

    fn d1(&self) -> i32 {
        Self::combine(self.z >> 1)
    }

    fn d2(&self) -> i32 {
        Self::combine(self.z >> 2)
    }

    fn decode(&self) -> (i32, i32, i32) {
        (self.d0(), self.d1(), self.d2())
    }

    /// Constructor.
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Z3 {
            z: Self::split(x.into()) | Self::split(y.into()) << 1 | Self::split(z.into()) << 2,
        }
    }

    fn partial_overlaps(a1: i32, a2: i32, b1: i32, b2: i32) -> bool {
        a1.max(b1) <= a2.min(b2)
    }
}

impl ZN for Z3 {
    const DIMENSIONS: i32 = 3;
    const BITS_PER_DIMENSION: i32 = 21;
    const TOTAL_BITS: i32 = 63;
    const MAX_MASK: i64 = 0x1f_ffff;

    fn split(value: i64) -> i64 {
        let mut x = value & Self::MAX_MASK;
        x = (x | x << 32) & 0x1f_0000_0000_ffff;
        x = (x | x << 16) & 0x1f_0000_ff00_00ff;
        x = (x | x << 8) & 0x100f_00f0_0f00_f00f;
        x = (x | x << 4) & 0x10c3_0c30_c30c_30c3;
        (x | x << 2) & 0x1249_2492_4924_9249
    }

    fn combine(z: i64) -> i32 {
        let mut x = z & 0x1249_2492_4924_9249;
        x = (x ^ (x >> 2)) & 0x10c3_0c30_c30c_30c3;
        x = (x ^ (x >> 4)) & 0x100f_00f0_0f00_f00f;
        x = (x ^ (x >> 8)) & 0x1f_0000_ff00_00ff;
        x = (x ^ (x >> 16)) & 0x1f_0000_0000_ffff;
        x = (x ^ (x >> 32)) & Self::MAX_MASK;
        x.try_into()
            .expect("The whole i64 fits into i32 because the bits have been combined.")
    }

    fn contains(range: ZRange, value: i64) -> bool {
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
        assert_eq!(Z3::new(i32::max_value(), 0, 0).decode(), (2097151, 0, 0));
        assert_eq!(
            Z3::new(i32::max_value(), 0, i32::max_value()).decode(),
            (2097151, 0, 2097151)
        );
    }

    #[quickcheck]
    fn test_encode_decode(x: u16, y: u16, z: u16) -> bool {
        Z3::new(x.into(), y.into(), z.into()).decode() == (x.into(), y.into(), z.into())
    }
}
