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

//! `ZRange` struct is a rectangle defined by the upper left and lower right corners.

/// z-order index aware rectangle defined by min (upper left) and max (lower right)
#[derive(Debug, PartialEq, Copy, Clone)]
pub struct ZRange {
    /// Upper left of Rectangle.
    pub min: u64,
    /// Lower right of Rectangle.
    pub max: u64,
}

impl ZRange {
    /// Midpoint between min and max.
    #[must_use]
    pub const fn mid(&self) -> u64 {
        self.min + ((self.max - self.min) >> 1)
    }

    /// Length between min and max. Saturates at `u64::MAX` for a range spanning
    /// the full `u64` domain, whose true length (`2^64`) is not representable.
    #[must_use]
    pub const fn length(&self) -> u64 {
        (self.max - self.min).saturating_add(1)
    }

    /// In index space, contains the bits value.
    #[must_use]
    pub const fn contains(&self, bits: u64) -> bool {
        bits >= self.min && bits <= self.max
    }

    /// Contains another `ZRange`.
    #[must_use]
    pub const fn contains_zrange(&self, r: ZRange) -> bool {
        self.contains(r.min) && self.contains(r.max)
    }

    /// Tests whether self and other overlap.
    #[must_use]
    pub const fn overlaps(&self, other: ZRange) -> bool {
        self.min <= other.max && other.min <= self.max
    }
}

#[cfg(kani)]
mod kani_proofs {
    use super::*;

    /// A well-formed range (`min <= max`) contains both of its own endpoints.
    #[kani::proof]
    fn contains_endpoints() {
        let min: u64 = kani::any();
        let max: u64 = kani::any();
        kani::assume(min <= max);
        let r = ZRange { min, max };
        assert!(r.contains(min));
        assert!(r.contains(max));
    }

    /// If `self` contains a well-formed sub-range, then it contains every value
    /// in that sub-range, not just the endpoints.
    #[kani::proof]
    fn contains_zrange_is_pointwise() {
        let min: u64 = kani::any();
        let max: u64 = kani::any();
        kani::assume(min <= max);
        let outer = ZRange { min, max };

        let r_min: u64 = kani::any();
        let r_max: u64 = kani::any();
        kani::assume(r_min <= r_max);
        let inner = ZRange {
            min: r_min,
            max: r_max,
        };
        kani::assume(outer.contains_zrange(inner));

        let bits: u64 = kani::any();
        kani::assume(bits >= r_min && bits <= r_max);
        assert!(outer.contains(bits));
    }

    /// `mid` of a well-formed range lies within the range (and never overflows).
    #[kani::proof]
    fn mid_within_range() {
        let min: u64 = kani::any();
        let max: u64 = kani::any();
        kani::assume(min <= max);
        let r = ZRange { min, max };
        let m = r.mid();
        assert!(m >= min && m <= max);
    }

    /// `length` of a well-formed range is at least 1 (and never overflows).
    #[kani::proof]
    fn length_at_least_one() {
        let min: u64 = kani::any();
        let max: u64 = kani::any();
        kani::assume(min <= max);
        let r = ZRange { min, max };
        assert!(r.length() >= 1);
    }

    /// Overlap is symmetric for well-formed ranges: if `a` overlaps `b`, then
    /// `b` overlaps `a`.
    #[kani::proof]
    fn overlaps_is_symmetric() {
        let a_min: u64 = kani::any();
        let a_max: u64 = kani::any();
        kani::assume(a_min <= a_max);
        let a = ZRange {
            min: a_min,
            max: a_max,
        };

        let b_min: u64 = kani::any();
        let b_max: u64 = kani::any();
        kani::assume(b_min <= b_max);
        let b = ZRange {
            min: b_min,
            max: b_max,
        };

        assert_eq!(a.overlaps(b), b.overlaps(a));
    }
}
