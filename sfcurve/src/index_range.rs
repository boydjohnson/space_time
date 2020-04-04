//! Contains trait `IndexRange` and concrete structs `CoveredRange` and
//! `OverlappingRange`. IndexRange has `Ord` so is sortable.
//!

use core::cmp::{Ord, Ordering};

/// Sortable Range trait.
pub trait IndexRange {
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

///
#[derive(Debug, PartialEq, Eq)]
pub struct CoveredRange {
    upper: i64,
    lower: i64,
}

fn cmp<T: IndexRange>(first: &T, other: &T) -> Ordering {
    let l_cmp = first.lower().cmp(&other.lower());
    if l_cmp != Ordering::Equal {
        return l_cmp;
    }
    let u_cmp = first.upper().cmp(&other.upper());
    if u_cmp != Ordering::Equal {
        return u_cmp;
    }
    Ordering::Equal
}

impl Ord for CoveredRange {
    fn cmp(&self, other: &Self) -> Ordering {
        cmp(self, &other)
    }
}

impl PartialOrd for CoveredRange {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
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

impl Ord for OverlappingRange {
    fn cmp(&self, other: &Self) -> Ordering {
        cmp(self, &other)
    }
}

impl PartialOrd for OverlappingRange {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
