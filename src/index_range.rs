//! Contains trait `IndexRange` and concrete structs `CoveredRange` and
//! `OverlappingRange`. IndexRange has `Ord` so is sortable.
//!

use core::cmp::{Ord, Ordering};

/// Sortable Range trait.
pub trait IndexRange: core::fmt::Debug {
    /// The lower index.
    fn lower(&self) -> i64;

    /// The upper index.
    fn upper(&self) -> i64;

    /// Contained.
    fn contained(&self) -> bool;

    /// Returns all three (lower, upper, contained) as a tuple.
    fn tuple(&self) -> (i64, i64, bool) {
        (
            <Self as IndexRange>::lower(&self),
            <Self as IndexRange>::upper(&self),
            self.contained(),
        )
    }
}

impl Ord for dyn IndexRange {
    fn cmp(&self, other: &Self) -> Ordering {
        let l_cmp = self.lower().cmp(&other.lower());
        if l_cmp != Ordering::Equal {
            return l_cmp;
        }
        let u_cmp = self.upper().cmp(&other.upper());
        if u_cmp != Ordering::Equal {
            return u_cmp;
        }
        Ordering::Equal
    }
}

impl PartialOrd for dyn IndexRange {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for dyn IndexRange {
    fn eq(&self, other: &Self) -> bool {
        self.lower() == other.lower() && self.upper() == other.upper()
    }
}

impl Eq for dyn IndexRange {}

///
#[derive(Debug, PartialEq, Eq)]
pub struct CoveredRange {
    upper: i64,
    lower: i64,
}

impl CoveredRange {
    /// Constructor.
    pub fn new(lower: i64, upper: i64) -> Self {
        CoveredRange { lower, upper }
    }
}

impl IndexRange for CoveredRange {
    fn upper(&self) -> i64 {
        self.upper
    }

    fn lower(&self) -> i64 {
        self.lower
    }

    fn contained(&self) -> bool {
        true
    }
}

/// An overlapping range.
#[derive(Debug, PartialEq, Eq)]
pub struct OverlappingRange {
    upper: i64,
    lower: i64,
}

impl OverlappingRange {
    /// Constructor.
    pub fn new(lower: i64, upper: i64) -> Self {
        OverlappingRange { lower, upper }
    }
}

impl IndexRange for OverlappingRange {
    fn upper(&self) -> i64 {
        self.upper
    }

    fn lower(&self) -> i64 {
        self.lower
    }

    fn contained(&self) -> bool {
        false
    }
}
