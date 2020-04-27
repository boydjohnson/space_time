//! `ZRange` struct is a rectangle defined by the upper left and lower right corners.
//!

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
        (self.max + self.min) >> 1
    }

    /// Length between min and max.
    #[must_use]
    pub const fn length(&self) -> u64 {
        self.max - self.min + 1
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
        self.contains(other.min) || self.contains(other.max)
    }
}
