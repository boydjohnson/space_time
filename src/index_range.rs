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

//! Contains trait `IndexRange` and concrete structs `CoveredRange` and
//! `OverlappingRange`. `IndexRange` has `Ord` so is sortable.

use core::cmp::{Ord, Ordering};

/// Sortable Range trait.
pub trait IndexRange: core::fmt::Debug {
    /// The lower index.
    fn lower(&self) -> u64;

    /// The upper index.
    fn upper(&self) -> u64;

    /// Contained.
    fn contained(&self) -> bool;

    /// Returns all three (lower, upper, contained) as a tuple.
    fn tuple(&self) -> (u64, u64, bool) {
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
    upper: u64,
    lower: u64,
}

impl CoveredRange {
    /// Constructor.
    #[must_use]
    pub fn new(lower: u64, upper: u64) -> Self {
        CoveredRange { upper, lower }
    }
}

impl IndexRange for CoveredRange {
    fn upper(&self) -> u64 {
        self.upper
    }

    fn lower(&self) -> u64 {
        self.lower
    }

    fn contained(&self) -> bool {
        true
    }
}

/// An overlapping range.
#[derive(Debug, PartialEq, Eq)]
pub struct OverlappingRange {
    upper: u64,
    lower: u64,
}

impl OverlappingRange {
    /// Constructor.
    #[must_use]
    pub fn new(lower: u64, upper: u64) -> Self {
        OverlappingRange { upper, lower }
    }
}

impl IndexRange for OverlappingRange {
    fn upper(&self) -> u64 {
        self.upper
    }

    fn lower(&self) -> u64 {
        self.lower
    }

    fn contained(&self) -> bool {
        false
    }
}
