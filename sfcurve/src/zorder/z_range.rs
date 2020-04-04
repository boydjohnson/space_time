//! ZRange struct is a rectangle defined by the lower left and upper right corners.
//!

/** z-order index aware rectangle defined by min (lower left) and max (upper right)
 *
 */
pub struct ZRange {
    min: i64,
    max: i64,
}

impl ZRange {
    /** Midpoint between min and max.
     *
     */
    pub const fn mid(&self) -> i64 {
        (self.max + self.min) >> 1
    }

    /** Length between min and max.
     *
     */
    pub const fn length(&self) -> i64 {
        self.max - self.min + 1
    }

    /** In index space, contains the bits value.
     *
     */
    pub const fn contains(&self, bits: i64) -> bool {
        bits >= self.min && bits <= self.max
    }

    /** Contains another `ZRange`.
     *
     */
    pub const fn contains_zrange(&self, r: ZRange) -> bool {
        self.contains(r.min) && self.contains(r.max)
    }

    /** Tests whether self and other overlap.
     *
     */
    pub const fn overlaps(&self, other: ZRange) -> bool {
        self.contains(other.min) || self.contains(other.max)
    }
}
