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

//! An N-Dimensional Z-Order Curve base class.

use crate::{
    index_range::{CoveredRange, IndexRange, OverlappingRange},
    zorder::z_range::ZRange,
};
use alloc::{boxed::Box, collections::VecDeque, vec, vec::Vec};

const DEFAULT_RECURSE: usize = 7;

const LEVEL_TERMINATOR: (Option<u64>, Option<u64>) = (None, None);

/// An N-Dimensional Z-Order Curve base class.
pub trait ZN {
    /// Number of Bits per Dimension.
    const BITS_PER_DIMENSION: u32;

    /// Number of Dimensions.
    const DIMENSIONS: u64;

    /// MAX Value of this Z-order.
    const MAX_MASK: u64;

    /// Total bits used. Usually bits_per_dim * dim.
    const TOTAL_BITS: u64;

    /// Number of quadrants in the quad/oct tree.
    const QUADRANTS: u32 = 2_u32.pow(Self::DIMENSIONS as u32);

    /// Insert (DIMENSIONS - 1) zeros between each bit to create a zvalue
    ///  from a single dimension.
    ///
    /// #Note:
    ///   - Only the first `BITS_PER_DIMENSION` can be considered.
    fn split(value: u32) -> u64;

    /// Combine every (Dimensions - 1) bits to re-create a single dimension. Opposite
    /// of split.
    fn combine(z: u64) -> u32;

    /// Tests whether range contains the value. Considers User space.
    fn contains(range: ZRange, value: u64) -> bool;

    /// Test whether range contains the value. Considers User space.
    #[must_use]
    fn contains_value(range: ZRange, value: ZRange) -> bool {
        Self::contains(range, value.min) && Self::contains(range, value.max)
    }

    /// Test whether range and value overlap. Considers User space.
    #[must_use]
    fn overlaps(range: ZRange, value: ZRange) -> bool;

    /// Compute the Z-index ranges that cover zbounds (Default values: precision = 64,
    /// `max_recurse` = 7, `max_ranges` = `usize::max_value()`).
    #[must_use]
    fn zranges_default<Z: ZN>(zbounds: &[ZRange]) -> Vec<Box<dyn IndexRange>> {
        Self::zranges::<Z>(zbounds, 64, Some(usize::max_value()), Some(DEFAULT_RECURSE))
    }

    /// Compute the Z-index ranges that cover zbounds.
    #[must_use]
    fn zranges<Z: ZN>(
        zbounds: &[ZRange],
        precision: u64,
        max_ranges: Option<usize>,
        max_recurse: Option<usize>,
    ) -> Vec<Box<dyn IndexRange>> {
        let mut ranges: Vec<Box<dyn IndexRange>> = Vec::with_capacity(100);

        let mut remaining: VecDeque<(Option<u64>, Option<u64>)> = VecDeque::with_capacity(100);

        let lcp = Self::longest_common_prefix(
            zbounds
                .iter()
                .flat_map(|b| vec![b.min, b.max])
                .collect::<Vec<u64>>()
                .as_slice(),
        );

        let mut offset = 64 - lcp.precision;

        check_value::<Z>(
            lcp.prefix,
            0,
            offset,
            zbounds,
            precision,
            &mut ranges,
            &mut remaining,
        );
        remaining.push_back(LEVEL_TERMINATOR);
        offset -= Self::DIMENSIONS;

        let mut level = 0;

        let max_recurse = max_recurse.unwrap_or(DEFAULT_RECURSE);
        let max_ranges = max_ranges.unwrap_or(usize::max_value());

        loop {
            let next = remaining.pop_front();

            match next {
                Some(LEVEL_TERMINATOR) => {
                    if !remaining.is_empty() {
                        level += 1;

                        if offset == 0 || level >= max_recurse {
                            bottom_out(&mut ranges, &mut remaining);
                        } else {
                            remaining.push_back(LEVEL_TERMINATOR);
                        }
                        offset -= Self::DIMENSIONS;
                    }
                }
                Some((Some(min), _)) => {
                    let prefix = min;
                    let mut quadrant = 0_u64;
                    while quadrant < Self::QUADRANTS.into() {
                        check_value::<Z>(
                            prefix,
                            quadrant,
                            offset,
                            zbounds,
                            precision,
                            &mut ranges,
                            &mut remaining,
                        );
                        quadrant += 1;
                    }
                    if ranges.len() + remaining.len() > max_ranges {
                        bottom_out(&mut ranges, &mut remaining);
                    }
                }
                _ => (),
            }

            if remaining.is_empty() {
                break;
            }
        }

        // All ranges found. Now reduce them by merging overlapping values.
        ranges.sort();

        let mut current: Option<Box<dyn IndexRange>> = None;
        let mut results = Vec::new();

        for range in ranges {
            if let Some(cur) = current.take() {
                if range.lower() <= cur.upper() + 1 {
                    let max = cur.upper().max(range.upper());
                    let min = cur.lower();
                    if cur.contained() && range.contained() {
                        current = Some(Box::new(CoveredRange::new(min, max)));
                    } else {
                        current = Some(Box::new(OverlappingRange::new(min, max)));
                    }
                } else {
                    results.push(cur);
                    current = Some(range);
                }
            } else {
                current = Some(range);
            }
        }
        if let Some(cur) = current {
            results.push(cur);
        }
        results
    }

    /// Compute the longest common binary prefix for a slice of i64s.
    ///
    /// # NOTE:
    ///   panics if `values.len() == 0`
    #[must_use]
    fn longest_common_prefix(values: &[u64]) -> ZPrefix {
        assert!(!values.is_empty());

        let mut bit_shift = Self::TOTAL_BITS - Self::DIMENSIONS;
        let mut head = values[0].wrapping_shr(bit_shift as u32);

        while values[1..]
            .iter()
            .all(|v| v.wrapping_shr(bit_shift as u32) == head)
        {
            bit_shift -= Self::DIMENSIONS;
            head = values[0].wrapping_shr(bit_shift as u32);
            if bit_shift == 0 {
                break;
            }
        }

        bit_shift += Self::DIMENSIONS;
        ZPrefix {
            prefix: values[0] & (u64::max_value().wrapping_shl(bit_shift as u32)),
            precision: 64 - bit_shift,
        }
    }
}

/// The longest common prefix for a group of z-indexes.
#[derive(Debug, PartialEq)]
pub struct ZPrefix {
    /// The common prefix.
    pub prefix: u64,
    /// The number of bits in common.
    pub precision: u64,
}

fn check_value<Z: ZN>(
    prefix: u64,
    quadrant: u64,
    offset: u64,
    zbounds: &[ZRange],
    precision: u64,
    ranges: &mut Vec<Box<dyn IndexRange>>,
    remaining: &mut VecDeque<(Option<u64>, Option<u64>)>,
) {
    let min = prefix | quadrant.wrapping_shl(offset as u32);
    let max = min | (1_u64.wrapping_shl(offset as u32) - 1);
    let quadrant_range = ZRange { min, max };

    if is_contained::<Z>(quadrant_range, zbounds) || offset < 64 - precision {
        ranges.push(Box::new(CoveredRange::new(min, max)));
    } else if is_overlapped::<Z>(quadrant_range, zbounds) {
        remaining.push_back((Some(min), Some(max)));
    }
}

fn bottom_out(
    ranges: &mut Vec<Box<dyn IndexRange>>,
    remaining: &mut VecDeque<(Option<u64>, Option<u64>)>,
) {
    while let Some((min, max)) = remaining.pop_front() {
        if let (Some(min), Some(max)) = (min, max) {
            ranges.push(Box::new(OverlappingRange::new(min, max)));
        }
    }
}

fn is_contained<Z: ZN>(range: ZRange, zbounds: &[ZRange]) -> bool {
    for bound in zbounds {
        if Z::contains_value(*bound, range) {
            return true;
        }
    }
    false
}

fn is_overlapped<Z: ZN>(range: ZRange, zbounds: &[ZRange]) -> bool {
    for bound in zbounds {
        if Z::overlaps(*bound, range) {
            return true;
        }
    }
    false
}
