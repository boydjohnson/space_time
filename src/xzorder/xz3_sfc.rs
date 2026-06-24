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

//! Space-Time filling curve for non-points

use crate::index_range::{CoveredRange, IndexRange, OverlappingRange};
use alloc::{boxed::Box, collections::VecDeque, vec, vec::Vec};
use num_integer::div_floor;
#[allow(unused_imports)]
use num_traits::Float;

/// An extended z-order curve for space-time indexing with non-points.
pub struct XZ3SFC {
    g: u32,
    x_min: f64,
    x_max: f64,
    y_min: f64,
    y_max: f64,
    z_min: f64,
    z_max: f64,
}

const LEVEL_TERMINATOR: Option<XElement> = None;

impl XZ3SFC {
    /// Maximum supported resolution. Above this, the `7 * (8.pow(g - i) - 1)`
    /// terms in `sequence_code` overflow `u64` (`7 * 8^21 > 2^64`).
    pub const MAX_G: u32 = 20;

    /// Create an 3D extended z-order curve in unprojected coordinates.
    ///
    /// # Panics
    ///
    /// Panics if `g > MAX_G` (20), the resolution above which the curve's
    /// index arithmetic overflows `u64`.
    #[must_use]
    pub fn wgs84(g: u32, z_min: f64, z_max: f64) -> Self {
        assert!(g <= Self::MAX_G, "resolution g must be <= {}", Self::MAX_G);
        XZ3SFC {
            g,
            x_min: -180.0,
            x_max: 180.0,
            y_min: -90.0,
            y_max: 90.0,
            z_min,
            z_max,
        }
    }

    /// General constructor for XZ3SFC.
    ///
    /// # Panics
    ///
    /// Panics if `g > MAX_G` (20).
    #[must_use]
    pub fn new(
        g: u32,
        x_min: f64,
        y_min: f64,
        z_min: f64,
        x_max: f64,
        y_max: f64,
        z_max: f64,
    ) -> Self {
        assert!(g <= Self::MAX_G, "resolution g must be <= {}", Self::MAX_G);
        XZ3SFC {
            g,
            x_min,
            x_max,
            y_min,
            y_max,
            z_min,
            z_max,
        }
    }

    fn x_size(&self) -> f64 {
        self.x_max - self.x_min
    }

    fn y_size(&self) -> f64 {
        self.y_max - self.y_min
    }

    fn z_size(&self) -> f64 {
        self.z_max - self.z_min
    }

    /// Compute the index for a bounding box with a time (z) component
    pub fn index(
        &self,
        x_min: f64,
        y_min: f64,
        z_min: f64,
        x_max: f64,
        y_max: f64,
        z_max: f64,
    ) -> u64 {
        let (nxmin, nymin, nzmin, nxmax, nymax, nzmax) =
            self.normalize(x_min, y_min, z_min, x_max, y_max, z_max);

        let max_dim = (nxmax - nxmin).max(nymax - nymin).max(nzmax - nzmin);

        let el_1 = max_dim.log(0.5).floor() as i32;

        let length = if el_1 as u32 >= self.g {
            self.g
        } else {
            let w2 = 0.5_f64.powi(el_1 + 1);
            if Self::predicate(nxmin, nxmax, w2)
                && Self::predicate(nymin, nymax, w2)
                && Self::predicate(nzmin, nzmax, w2)
            {
                (el_1 + 1) as u32
            } else {
                el_1 as u32
            }
        };

        self.sequence_code(nxmin, nymin, nzmin, length)
    }

    fn predicate(min: f64, max: f64, w2: f64) -> bool {
        max <= (min / w2).floor() * w2 + (2.0 * w2)
    }

    /// Compute the index range that are contained or overlap the bounding box.
    #[allow(clippy::too_many_arguments)]
    pub fn ranges(
        &self,
        xmin: f64,
        ymin: f64,
        zmin: f64,
        xmax: f64,
        ymax: f64,
        zmax: f64,
        max_ranges: Option<u16>,
    ) -> Vec<Box<dyn IndexRange>> {
        let windows = {
            let (nxmin, nymin, nzmin, nxmax, nymax, nzmax) =
                self.normalize(xmin, ymin, zmin, xmax, ymax, zmax);
            &[QueryWindow {
                x_min: nxmin,
                y_min: nymin,
                z_min: nzmin,
                x_max: nxmax,
                y_max: nymax,
                z_max: nzmax,
            }]
        };

        let range_stop = max_ranges.unwrap_or(u16::MAX);
        self.ranges_impl(windows, range_stop)
    }

    fn ranges_impl(&self, query: &[QueryWindow], range_stop: u16) -> Vec<Box<dyn IndexRange>> {
        let mut ranges = Vec::with_capacity(100);

        let mut remaining = VecDeque::with_capacity(100);

        for el in XElement::level_one_elements() {
            remaining.push_back(Some(el));
        }
        remaining.push_back(LEVEL_TERMINATOR);

        let mut level = 1;

        while level < self.g && !remaining.is_empty() && ranges.len() < range_stop.into() {
            match remaining.pop_front() {
                Some(LEVEL_TERMINATOR) if !remaining.is_empty() => {
                    level += 1;
                    remaining.push_back(LEVEL_TERMINATOR);
                }
                Some(Some(oct)) => {
                    self.check_value(&oct, level, query, &mut ranges, &mut remaining);
                }
                _ => (),
            }
        }

        while let Some(el) = remaining.pop_front() {
            if let Some(oct) = el {
                let (min, max) =
                    self.sequence_interval(oct.x_min, oct.y_min, oct.z_min, level, false);
                ranges.push(Box::new(OverlappingRange::new(min, max)));
            } else {
                level += 1;
            }
        }

        ranges.sort();

        let mut current: Option<Box<dyn IndexRange>> = None;
        let mut results = vec![];
        for range in ranges {
            if let Some(cur) = current {
                if range.lower() <= cur.upper().saturating_add(1) {
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

        if let Some(current) = current {
            results.push(current);
        }
        results
    }

    fn is_contained(oct: &XElement, query: &[QueryWindow]) -> bool {
        for q in query {
            if oct.is_contained(q) {
                return true;
            }
        }
        false
    }

    fn is_overlapped(oct: &XElement, query: &[QueryWindow]) -> bool {
        for q in query {
            if oct.is_overlapped(q) {
                return true;
            }
        }
        false
    }

    fn check_value(
        &self,
        oct: &XElement,
        level: u32,
        query: &[QueryWindow],
        ranges: &mut Vec<Box<dyn IndexRange>>,
        remaining: &mut VecDeque<Option<XElement>>,
    ) {
        if Self::is_contained(oct, query) {
            let (min, max) = self.sequence_interval(oct.x_min, oct.y_min, oct.z_min, level, false);
            ranges.push(Box::new(CoveredRange::new(min, max)));
        } else if Self::is_overlapped(oct, query) {
            let (min, max) = self.sequence_interval(oct.x_min, oct.y_min, oct.z_min, level, true);
            ranges.push(Box::new(OverlappingRange::new(min, max)));
            for el in oct.children() {
                remaining.push_back(Some(el));
            }
        }
    }

    fn normalize(
        &self,
        x_min: f64,
        y_min: f64,
        z_min: f64,
        x_max: f64,
        y_max: f64,
        z_max: f64,
    ) -> (f64, f64, f64, f64, f64, f64) {
        (
            (x_min - self.x_min) / self.x_size(),
            (y_min - self.y_min) / self.y_size(),
            (z_min - self.z_min) / self.z_size(),
            (x_max - self.x_min) / self.x_size(),
            (y_max - self.y_min) / self.y_size(),
            (z_max - self.z_min) / self.z_size(),
        )
    }

    /// Curve-code offset contributed by octant `q` (`0..=7`) at remaining
    /// resolution `k`: the number of cells skipped by stepping past octants
    /// `0..q` of a node whose subtree spans `k` more levels. Shared by
    /// `sequence_code` and `sequence_interval` so the kani overflow proof
    /// exercises the exact arithmetic used in production. Does not overflow for
    /// `k <= MAX_G` (see `code_offset_doesnt_overflow`).
    fn code_offset(q: u64, k: u32) -> u64 {
        div_floor(q * (8_u64.pow(k) - 1), 7)
    }

    fn sequence_code(&self, x: f64, y: f64, z: f64, length: u32) -> u64 {
        let mut x_min = 0.0;
        let mut y_min = 0.0;
        let mut z_min = 0.0;
        let mut x_max = 1.0;
        let mut y_max = 1.0;
        let mut z_max = 1.0;

        let mut cs = 0_u64;

        for i in 0..length {
            let x_center = (x_min + x_max) / 2.0;
            let y_center = (y_min + y_max) / 2.0;
            let z_center = (z_min + z_max) / 2.0;

            match (x < x_center, y < y_center, z < z_center) {
                (true, true, true) => {
                    cs += 1;
                    x_max = x_center;
                    y_max = y_center;
                    z_max = z_center;
                }
                (false, true, true) => {
                    cs += 1 + Self::code_offset(1, self.g - i);
                    x_min = x_center;
                    y_max = y_center;
                    z_max = z_center;
                }
                (true, false, true) => {
                    cs += 1 + Self::code_offset(2, self.g - i);
                    x_max = x_center;
                    y_min = y_center;
                    z_max = z_center;
                }
                (false, false, true) => {
                    cs += 1 + Self::code_offset(3, self.g - i);
                    x_min = x_center;
                    y_min = y_center;
                    z_max = z_center;
                }
                (true, true, false) => {
                    cs += 1 + Self::code_offset(4, self.g - i);
                    x_max = x_center;
                    y_max = y_center;
                    z_min = z_center;
                }
                (false, true, false) => {
                    cs += 1 + Self::code_offset(5, self.g - i);
                    x_min = x_center;
                    y_max = y_center;
                    z_min = z_center;
                }
                (true, false, false) => {
                    cs += 1 + Self::code_offset(6, self.g - i);
                    x_max = x_center;
                    y_min = y_center;
                    z_min = z_center;
                }
                (false, false, false) => {
                    cs += 1 + Self::code_offset(7, self.g - i);
                    x_min = x_center;
                    y_min = y_center;
                    z_min = z_center;
                }
            }
        }
        cs
    }

    fn sequence_interval(&self, x: f64, y: f64, z: f64, length: u32, partial: bool) -> (u64, u64) {
        let min = self.sequence_code(x, y, z, length);

        let max = if partial {
            min
        } else {
            min + Self::code_offset(1, self.g - length + 1)
        };

        (min, max)
    }
}

struct QueryWindow {
    x_min: f64,
    y_min: f64,
    z_min: f64,
    x_max: f64,
    y_max: f64,
    z_max: f64,
}

#[derive(Debug, PartialEq)]
struct XElement {
    x_min: f64,
    y_min: f64,
    z_min: f64,
    x_max: f64,
    y_max: f64,
    z_max: f64,
    length: f64,
}

impl XElement {
    fn new(
        x_min: f64,
        y_min: f64,
        z_min: f64,
        x_max: f64,
        y_max: f64,
        z_max: f64,
        length: f64,
    ) -> Self {
        XElement {
            x_min,
            y_min,
            z_min,
            x_max,
            y_max,
            z_max,
            length,
        }
    }

    fn xext(&self) -> f64 {
        self.x_max + self.length
    }

    fn yext(&self) -> f64 {
        self.y_max + self.length
    }

    fn zext(&self) -> f64 {
        self.z_max + self.length
    }

    fn level_one_elements() -> Vec<XElement> {
        XElement::new(0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0).children()
    }

    fn is_contained(&self, window: &QueryWindow) -> bool {
        window.x_min <= self.x_min
            && window.y_min <= self.y_min
            && window.z_min <= self.z_min
            && window.x_max >= self.xext()
            && window.y_max >= self.yext()
            && window.z_max >= self.zext()
    }

    fn is_overlapped(&self, window: &QueryWindow) -> bool {
        window.x_max >= self.x_min
            && window.y_max >= self.y_min
            && window.z_max >= self.z_min
            && window.x_min <= self.xext()
            && window.y_min <= self.yext()
            && window.z_min <= self.zext()
    }

    fn children(&self) -> Vec<XElement> {
        let x_center = (self.x_min + self.x_max) / 2.0;
        let y_center = (self.y_min + self.y_max) / 2.0;
        let z_center = (self.z_min + self.z_max) / 2.0;
        let len = self.length / 2.0;

        vec![
            XElement {
                x_min: self.x_min,
                x_max: x_center,
                y_min: self.y_min,
                y_max: y_center,
                z_min: self.z_min,
                z_max: z_center,
                length: len,
            },
            XElement {
                x_min: x_center,
                x_max: self.x_max,
                y_min: self.y_min,
                y_max: y_center,
                z_min: self.z_min,
                z_max: z_center,
                length: len,
            },
            XElement {
                x_min: self.x_min,
                x_max: x_center,
                y_min: y_center,
                y_max: self.y_max,
                z_min: self.z_min,
                z_max: z_center,
                length: len,
            },
            XElement {
                x_min: x_center,
                x_max: self.x_max,
                y_min: y_center,
                y_max: self.y_max,
                z_min: self.z_min,
                z_max: z_center,
                length: len,
            },
            XElement {
                x_min: self.x_min,
                x_max: x_center,
                y_min: self.y_min,
                y_max: y_center,
                z_min: z_center,
                z_max: self.z_max,
                length: len,
            },
            XElement {
                x_min: x_center,
                x_max: self.x_max,
                y_min: self.y_min,
                y_max: y_center,
                z_min: z_center,
                z_max: self.z_max,
                length: len,
            },
            XElement {
                x_min: self.x_min,
                x_max: x_center,
                y_min: y_center,
                y_max: self.y_max,
                z_min: z_center,
                z_max: self.z_max,
                length: len,
            },
            XElement {
                x_min: x_center,
                x_max: self.x_max,
                y_min: y_center,
                y_max: self.y_max,
                z_min: z_center,
                z_max: self.z_max,
                length: len,
            },
        ]
    }
}

#[cfg(kani)]
mod kani_proofs {
    use super::*;

    /// `XZ3SFC::code_offset` — the single arithmetic helper behind every
    /// `sequence_code`/`sequence_interval` curve-code term — never overflows
    /// `u64` under the `g <= MAX_G` bound enforced by the constructors. The
    /// resolution argument `k` (`g - i` with `i` in `0..length <= g`, or
    /// `g - length + 1`) lies in `1..=g`, and the octant `q` in `0..=7`, so the
    /// proof ranges over every value reachable in production.
    #[kani::proof]
    #[kani::unwind(33)]
    fn code_offset_doesnt_overflow() {
        let g: u32 = kani::any();
        kani::assume(g <= XZ3SFC::MAX_G);

        let k: u32 = kani::any();
        kani::assume(k >= 1 && k <= g);

        let q: u64 = kani::any();
        kani::assume(q <= 7);

        let _ = XZ3SFC::code_offset(q, k);
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_bounding_boxes_query_polygon() {
        let sfc = XZ3SFC::wgs84(12, 0.0, 13000.0);

        let polygon = sfc.index(10.0, 10.0, 1000.0, 12.0, 12.0, 1000.0);

        let containing = [
            (9.0, 9.0, 900.0, 13.0, 13.0, 1100.0),
            (-180.0, -90.0, 900.0, 180.0, 90.0, 1100.0),
            (0.0, 0.0, 900.0, 180.0, 90.0, 1100.0),
            (0.0, 0.0, 900.0, 20.0, 20.0, 1100.0),
        ];

        let overlapping = [
            (11.0, 11.0, 900.0, 13.0, 13.0, 1100.0),
            (9.0, 9.0, 900.0, 11.0, 11.0, 1100.0),
            (10.5, 10.5, 900.0, 11.5, 11.5, 1100.0),
            (11.0, 11.0, 900.0, 11.0, 11.0, 1100.0),
        ];

        let disjoint = [
            (-180.0, -90.0, 900.0, 8.0, 8.0, 1100.0),
            (0.0, 0.0, 900.0, 8.0, 8.0, 1100.0),
            (9.0, 9.0, 900.0, 9.5, 9.5, 1100.0),
            (20.0, 20.0, 900.0, 180.0, 90.0, 1100.0),
        ];

        for bbox in &[containing, overlapping].concat() {
            let ranges = sfc.ranges(bbox.0, bbox.1, bbox.2, bbox.3, bbox.4, bbox.5, Some(10000));

            assert!(ranges
                .iter()
                .any(|r| r.lower() <= polygon && polygon <= r.upper()));
        }

        for bbox in &disjoint {
            let ranges = sfc.ranges(bbox.0, bbox.1, bbox.2, bbox.3, bbox.4, bbox.5, Some(10000));

            assert!(!ranges
                .iter()
                .any(|r| r.lower() <= polygon && polygon <= r.upper()));
        }
    }

    #[test]
    fn test_indexing() {
        let sfc = XZ3SFC::wgs84(12, 0.0, 100000.0);

        assert_eq!(
            sfc.index(-80.0, -45.0, 1000.0, -78.8, -40.0, 1000.0),
            3_681_700_138
        );

        assert_eq!(
            sfc.index(-80.0, -45.0, 2000.0, -78.8, -40.0, 2000.0),
            3_682_898_510
        );

        assert_eq!(
            sfc.index(80.0, 25.0, 2000.0, 87.8, 40.0, 2000.0),
            29_930_553_347
        );
    }

    #[test]
    fn test_queries() {
        let sfc = XZ3SFC::wgs84(12, 0.0, 100_000.0);

        let ranges = sfc.ranges(-80.0, -45.0, 900.0, -78.8, -40.0, 1100.0, None);

        assert_eq!(ranges.len(), 912);

        assert_eq!(ranges.first().map(|r| r.lower()), Some(1));

        assert_eq!(ranges.last().map(|r| r.upper()), Some(3_682_578_823));
    }
}

#[cfg(test)]
mod range_query_tests {
    use super::*;
    use quickcheck_macros::quickcheck;

    /// `index` must not panic on the float→int cast edge cases: a degenerate
    /// point box (`max_dim == 0`, where `log(0.5)` is `+inf` and the `el_1`
    /// cast saturates to `i32::MAX`), the full-extent box (`max_dim == 1`), and
    /// a sub-cell box.
    #[test]
    fn index_handles_cast_edge_cases() {
        let sfc = XZ3SFC::wgs84(12, 0.0, 100.0);
        let _ = sfc.index(10.0, 20.0, 5.0, 10.0, 20.0, 5.0); // point box -> length = g
        let _ = sfc.index(-180.0, -90.0, 0.0, 180.0, 90.0, 100.0); // full extent
        let _ = sfc.index(-179.999_9, -89.999_9, 0.0, -179.999_8, -89.999_8, 0.01);
    }

    /// Every `(code, element)` in the octree down to depth `g`.
    fn enumerate(sfc: &XZ3SFC) -> Vec<(u64, XElement)> {
        let mut out = Vec::new();
        let mut stack: Vec<(XElement, u32)> = XElement::level_one_elements()
            .into_iter()
            .map(|e| (e, 1))
            .collect();
        while let Some((e, level)) = stack.pop() {
            let code = sfc.sequence_code(e.x_min, e.y_min, e.z_min, level);
            if level < sfc.g {
                for c in e.children() {
                    stack.push((c, level + 1));
                }
            }
            out.push((code, e));
        }
        out
    }

    /// End-to-end property for the XZ3 range engine. For any query window and
    /// any `range_stop` (including the small values that force early
    /// `bottom_out`), `ranges_impl` must be
    ///
    ///  * COMPLETE — every element whose (enlarged) extent overlaps the query is covered
    ///    by some returned range, so no candidate object is missed, and
    ///  * SOUND — every element whose code lands in a `CoveredRange` is fully contained
    ///    in the query (overlapping ranges may spill, covered ones may not).
    ///
    /// Driven in normalized `[0, 1]` space with dyadic query coordinates so all
    /// element-boundary comparisons are exact in `f64`.
    #[quickcheck]
    fn xz3_ranges_complete_and_sound(
        g_seed: u8,
        coords: (u8, u8, u8, u8, u8, u8),
        range_stop: u8,
    ) -> bool {
        let g = u32::from(g_seed % 3) + 1; // 1..=3 (octree stays small)
        let sfc = XZ3SFC::wgs84(g, 0.0, 1.0);

        let denom = f64::from(1u32 << g); // 2^g
        let to_unit = |v: u8| f64::from(u32::from(v) % ((1u32 << g) + 1)) / denom;
        let (x0, y0, z0, x1, y1, z1) = coords;
        let (mut x_min, mut x_max) = (to_unit(x0), to_unit(x1));
        if x_min > x_max {
            core::mem::swap(&mut x_min, &mut x_max);
        }
        let (mut y_min, mut y_max) = (to_unit(y0), to_unit(y1));
        if y_min > y_max {
            core::mem::swap(&mut y_min, &mut y_max);
        }
        let (mut z_min, mut z_max) = (to_unit(z0), to_unit(z1));
        if z_min > z_max {
            core::mem::swap(&mut z_min, &mut z_max);
        }
        let window = QueryWindow {
            x_min,
            y_min,
            z_min,
            x_max,
            y_max,
            z_max,
        };

        // `0` means "no limit"; otherwise an aggressive cap that forces bottom-out.
        let range_stop = if range_stop == 0 {
            u16::MAX
        } else {
            u16::from(range_stop)
        };
        let ranges = sfc.ranges_impl(core::slice::from_ref(&window), range_stop);

        let elements = enumerate(&sfc);

        // Completeness.
        for (code, e) in &elements {
            if e.is_overlapped(&window)
                && !ranges
                    .iter()
                    .any(|r| r.lower() <= *code && *code <= r.upper())
            {
                return false;
            }
        }

        // Soundness of covered ranges.
        for (code, e) in &elements {
            for r in &ranges {
                if r.contained()
                    && r.lower() <= *code
                    && *code <= r.upper()
                    && !e.is_contained(&window)
                {
                    return false;
                }
            }
        }

        true
    }
}
