//! An N-Dimensional Z-Order Curve base class.

use crate::index_range::{CoveredRange, IndexRange, OverlappingRange};
use crate::zorder::z_range::ZRange;
use alloc::boxed::Box;
use alloc::collections::VecDeque;
use alloc::vec::Vec;

const DEFAULT_RECURSE: usize = 7;

const LEVEL_TERMINATOR: (i64, i64) = (-1, -1);

/// An N-Dimensional Z-Order Curve base class.
pub trait ZN {
    /// Number of Bits per Dimension.
    const BITS_PER_DIMENSION: i32;

    /// Number of Dimensions.
    const DIMENSIONS: i32;

    /// MAX Value of this Z-order.
    const MAX_MASK: i64;

    /// Total bits used. Usually bits_per_dim * dim.
    const TOTAL_BITS: i32;

    /// Number of quadrants in the quad/oct tree.
    const QUADRANTS: i32 = 2_i32.pow(Self::DIMENSIONS as u32);

    /// Insert (DIMENSIONS - 1) zeros between each bit to create a zvalue
    ///  from a single dimension.
    ///
    /// #Note:
    ///   - Only the first BITS_PER_DIMENSION can be considered.
    fn split(value: i64) -> i64;

    /// Combine every (Dimensions - 1) bits to re-create a single dimension. Opposite
    /// of split.
    fn combine(z: i64) -> i32;

    /// Tests whether range contains the value. Considers User space.
    fn contains(range: ZRange, value: i64) -> bool;

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
        precision: i32,
        max_ranges: Option<usize>,
        max_recurse: Option<usize>,
    ) -> Vec<Box<dyn IndexRange>> {
        let mut ranges: Vec<Box<dyn IndexRange>> = Vec::with_capacity(100);

        let mut remaining: VecDeque<(i64, i64)> = VecDeque::with_capacity(100);

        let lcp = Self::longest_common_prefix(
            zbounds
                .iter()
                .flat_map(|b| {
                    let mut arr = Vec::with_capacity(2);
                    arr.push(b.min);
                    arr.push(b.max);
                    arr
                })
                .collect::<Vec<i64>>()
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
                        offset -= Self::DIMENSIONS;
                        if level >= max_recurse || offset < 0 {
                            bottom_out(&mut ranges, &mut remaining);
                        } else {
                            remaining.push_back(LEVEL_TERMINATOR);
                        }
                    }
                }
                Some((min, _)) => {
                    let prefix = min;
                    let mut quadrant = 0_i64;
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
                None => (),
            }

            if remaining.is_empty() {
                break;
            }
        }

        // All ranges found. Now reduce them by merging overlapping values.
        ranges.sort();

        let mut current = if ranges.is_empty() {
            None
        } else {
            Some(ranges.remove(0))
        };
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
    fn longest_common_prefix(values: &[i64]) -> ZPrefix {
        assert!(!values.is_empty());

        let mut bit_shift = Self::TOTAL_BITS - Self::DIMENSIONS;
        let mut head = values[0].wrapping_shr(bit_shift as u32);

        while values[1..]
            .iter()
            .all(|v| v.wrapping_shr(bit_shift as u32) == head && bit_shift > -1)
        {
            bit_shift -= Self::DIMENSIONS;
            head = values[0].wrapping_shr(bit_shift as u32);
        }

        bit_shift += Self::DIMENSIONS;
        ZPrefix {
            prefix: values[0] & (i64::max_value().wrapping_shl(bit_shift as u32)),
            precision: 64 - bit_shift,
        }
    }
}

/// The longest common prefix for a group of z-indexes.
#[derive(Debug, PartialEq)]
pub struct ZPrefix {
    /// The common prefix.
    pub prefix: i64,
    /// The number of bits in common.
    pub precision: i32,
}

fn check_value<Z: ZN>(
    prefix: i64,
    quadrant: i64,
    offset: i32,
    zbounds: &[ZRange],
    precision: i32,
    ranges: &mut Vec<Box<dyn IndexRange>>,
    remaining: &mut VecDeque<(i64, i64)>,
) {
    let min = prefix | quadrant.wrapping_shl(offset as u32);
    let max = min | (1_i64.wrapping_shl(offset as u32) - 1);
    let quadrant_range = ZRange {
        min: min as i64,
        max: max as i64,
    };

    if is_contained::<Z>(quadrant_range, zbounds) || offset < 64 - precision {
        ranges.push(Box::new(CoveredRange::new(min as i64, max as i64)));
    } else if is_overlapped::<Z>(quadrant_range, zbounds) {
        remaining.push_back((min as i64, max as i64));
    }
}

fn bottom_out(ranges: &mut Vec<Box<dyn IndexRange>>, remaining: &mut VecDeque<(i64, i64)>) {
    while let Some((min, max)) = remaining.pop_front() {
        if (min, max) != LEVEL_TERMINATOR {
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
