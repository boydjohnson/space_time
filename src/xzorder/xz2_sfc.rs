//! SpaceFillingCurve for storing non-point features based on a bounding box.

use crate::index_range::{CoveredRange, IndexRange, OverlappingRange};
use alloc::boxed::Box;
use alloc::collections::VecDeque;
use alloc::vec;
use alloc::vec::Vec;
use num_integer::div_floor;
#[allow(unused_imports)]
use num_traits::Float;

/// Z-order curve implementation for non-point features.
///
/// Based on [geomesa-z3 scala implementation](https://github.com/locationtech/geomesa/blob/771777d3a9716b04f7dcd27a6b7d1bb822a1b5a7/geomesa-z3/src/main/scala/org/locationtech/geomesa/curve/XZ2SFC.scala)
/// which is based on 'XZ-Ordering: A Space Filling Curve for Objects
/// with Spatial Extension' by Christian Bohm, Gerald Klump, and Hans-Peter Kriegel
pub struct XZ2SFC {
    g: u32,
    x_min: f64,
    x_max: f64,
    y_min: f64,
    y_max: f64,
}

impl XZ2SFC {
    fn x_size(&self) -> f64 {
        self.x_max - self.x_min
    }

    fn y_size(&self) -> f64 {
        self.y_max - self.y_min
    }

    /// An `XZ2SFC` for unprojected coordinates.
    #[must_use]
    pub fn wgs84(g: u32) -> Self {
        XZ2SFC {
            g,
            x_min: -180.0,
            x_max: 180.0,
            y_min: -90.0,
            y_max: 90.0,
        }
    }

    /// Return the index for a bounding box.
    #[must_use]
    pub fn index(&self, xmin: f64, ymin: f64, xmax: f64, ymax: f64) -> u64 {
        let (nxmin, nymin, nxmax, nymax) = self.normalize(xmin, ymin, xmax, ymax);

        let max_dim = (nxmax - nxmin).max(nymax - nymin);

        // This is a slightly different construction but same value as geomesa.
        let el_1 = max_dim.log(0.5).floor() as i32;

        let length: u32 = if el_1 as u32 >= self.g {
            self.g
        } else {
            let w2 = 0.5_f64.powi(el_1 + 1);

            if Self::predicate(nxmin, nxmax, w2) && Self::predicate(nxmin, nxmax, w2) {
                (el_1 + 1) as u32
            } else {
                el_1 as u32
            }
        };

        self.sequence_code(nxmin, nymin, length)
    }

    fn predicate(min: f64, max: f64, w2: f64) -> bool {
        max <= (min / w2).floor() * w2 + 2.0 * w2
    }

    /// Compute that index ranges that are contained or overlap the bounding box.
    pub fn ranges(
        &self,
        xmin: f64,
        ymin: f64,
        xmax: f64,
        ymax: f64,
        max_ranges: Option<u16>,
    ) -> Vec<Box<dyn IndexRange>> {
        let windows = {
            let (nxmin, nymin, nxmax, nymax) = self.normalize(xmin, ymin, xmax, ymax);
            &[QueryWindow {
                xmin: nxmin,
                ymin: nymin,
                xmax: nxmax,
                ymax: nymax,
            }]
        };

        let range_stop = max_ranges.unwrap_or(u16::MAX);

        self.ranges_impl(windows, range_stop)
    }

    fn ranges_impl(&self, query: &[QueryWindow], range_stop: u16) -> Vec<Box<dyn IndexRange>> {
        let mut ranges: Vec<Box<dyn IndexRange>> = Vec::with_capacity(100);

        let mut remaining: VecDeque<Option<XElement>> = VecDeque::with_capacity(100);

        for el in XElement::level_one_elements() {
            remaining.push_back(Some(el));
        }
        remaining.push_back(LEVEL_TERMINATOR);

        let mut level: u32 = 1;

        while level < self.g && !remaining.is_empty() && ranges.len() < range_stop.into() {
            match remaining.pop_front() {
                Some(LEVEL_TERMINATOR) => {
                    if !remaining.is_empty() {
                        level += 1;
                        remaining.push_back(LEVEL_TERMINATOR);
                    }
                }
                Some(element) => {
                    self.check_value(element, level, query, &mut ranges, &mut remaining)
                }
                _ => (),
            }
        }

        while let Some(quad) = remaining.pop_front() {
            if let Some(quad) = quad {
                let (min, max) = self.sequence_interval(quad.xmin, quad.ymin, level, false);
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

        if let Some(current) = current {
            results.push(current);
        }

        results
    }

    fn sequence_code(&self, x: f64, y: f64, length: u32) -> u64 {
        let mut xmin = 0.0;
        let mut ymin = 0.0;
        let mut xmax = 1.0;
        let mut ymax = 1.0;

        let mut cs = 0_u64;

        for i in 0_u32..length {
            let x_center = (xmin + xmax) / 2.0;
            let y_center = (ymin + ymax) / 2.0;

            match (x < x_center, y < y_center) {
                (true, true) => {
                    cs += 1;
                    xmax = x_center;
                    ymax = y_center;
                }
                (false, true) => {
                    cs += 1 + div_floor(4_u64.pow(self.g as u32 - i) - 1_u64, 3);
                    xmin = x_center;
                    ymax = y_center;
                }
                (true, false) => {
                    cs += 1 + div_floor(2 * (4_u64.pow(self.g as u32 - i) - 1_u64), 3);
                    xmax = x_center;
                    ymin = y_center;
                }
                (false, false) => {
                    cs += 1 + div_floor(3 * 4_u64.pow(self.g as u32 - i) - 1_u64, 3);
                    xmin = x_center;
                    ymin = y_center;
                }
            }
        }
        cs
    }

    fn check_value(
        &self,
        quad: Option<XElement>,
        level: u32,
        query: &[QueryWindow],
        ranges: &mut Vec<Box<dyn IndexRange>>,
        remaining: &mut VecDeque<Option<XElement>>,
    ) {
        if let Some(quad) = quad {
            if Self::is_contained(quad, query) {
                let (min, max) = self.sequence_interval(quad.xmin, quad.ymin, level, false);
                ranges.push(Box::new(CoveredRange::new(min, max)));
            } else if Self::is_overlapped(quad, query) {
                let (min, max) = self.sequence_interval(quad.xmin, quad.ymin, level, true);
                ranges.push(Box::new(OverlappingRange::new(min, max)));
                for el in quad.children() {
                    remaining.push_back(Some(el));
                }
            }
        }
    }

    fn is_contained(quad: XElement, query: &[QueryWindow]) -> bool {
        for q in query {
            if quad.is_contained(q) {
                return true;
            }
        }
        false
    }

    fn is_overlapped(quad: XElement, query: &[QueryWindow]) -> bool {
        for q in query {
            if quad.overlaps(q) {
                return true;
            }
        }
        false
    }

    fn sequence_interval(&self, x: f64, y: f64, length: u32, partial: bool) -> (u64, u64) {
        let min = self.sequence_code(x, y, length);

        let max = if partial {
            min
        } else {
            min + div_floor(4_u64.pow(self.g - length + 1) - 1, 3)
        };

        (min, max)
    }

    fn normalize(&self, x_min: f64, y_min: f64, x_max: f64, y_max: f64) -> (f64, f64, f64, f64) {
        assert!(x_min <= x_max && y_min <= y_max);
        assert!(
            x_min >= self.x_min
                && x_max <= self.x_max
                && y_min >= self.y_min
                && y_max <= self.y_max
        );

        (
            (x_min - self.x_min) / self.x_size(),
            (y_min - self.y_min) / self.y_size(),
            (x_max - self.x_min) / self.x_size(),
            (y_max - self.y_min) / self.y_size(),
        )
    }
}

const LEVEL_TERMINATOR: Option<XElement> = None;

#[derive(Debug, Clone, Copy)]
struct QueryWindow {
    pub xmin: f64,
    pub ymin: f64,
    pub xmax: f64,
    pub ymax: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct XElement {
    xmin: f64,
    ymin: f64,
    xmax: f64,
    ymax: f64,
    length: f64,
}

impl XElement {
    const fn new(xmin: f64, ymin: f64, xmax: f64, ymax: f64, length: f64) -> Self {
        XElement {
            xmin,
            ymin,
            xmax,
            ymax,
            length,
        }
    }

    fn xext(&self) -> f64 {
        self.xmax + self.length
    }

    fn yext(&self) -> f64 {
        self.ymax + self.length
    }

    fn is_contained(&self, window: &QueryWindow) -> bool {
        window.xmin <= self.xmin
            && window.ymin <= self.ymin
            && window.xmax >= self.xext()
            && window.ymax >= self.yext()
    }

    fn overlaps(&self, window: &QueryWindow) -> bool {
        window.xmax >= self.xmin
            && window.ymax >= self.ymin
            && window.xmin <= self.xext()
            && window.ymin <= self.yext()
    }

    fn level_one_elements() -> Vec<XElement> {
        XElement::new(0.0, 0.0, 1.0, 1.0, 1.0).children()
    }

    fn children(&self) -> Vec<XElement> {
        let x_center = (self.xmin + self.xmax) / 2.0;
        let y_center = (self.ymin + self.ymax) / 2.0;
        let len = self.length / 2.0;

        vec![
            XElement::new(self.xmin, self.ymin, x_center, y_center, len),
            XElement::new(x_center, self.ymin, self.xmax, y_center, len),
            XElement::new(self.xmin, y_center, x_center, self.ymax, len),
            XElement::new(x_center, y_center, self.xmax, self.ymax, len),
        ]
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_query_bounding_boxes() {
        let sfc = XZ2SFC::wgs84(12);
        let polygon = sfc.index(10.0, 10.0, 12.0, 12.0);

        let containing = [
            (9.0, 9.0, 13.0, 13.0),
            (-180.0, -90.0, 180.0, 90.0),
            (0.0, 0.0, 180.0, 90.0),
            (0.0, 0.0, 20.0, 20.0),
        ];
        let overlapping = [
            (11.0, 11.0, 13.0, 13.0),
            (9.0, 9.0, 11.0, 11.0),
            (10.5, 10.5, 11.5, 11.5),
            (11.0, 11.0, 11.0, 11.0),
        ];
        let disjoint = [
            (-180.0, -90.0, 8.0, 8.0),
            (0.0, 0.0, 8.0, 8.0),
            (9.0, 9.0, 9.5, 9.5),
            (20.0, 20.0, 180.0, 90.0),
        ];

        for bbox in &[containing, overlapping].concat() {
            let ranges = sfc.ranges(bbox.0, bbox.1, bbox.2, bbox.3, None);
            assert!(ranges
                .iter()
                .any(|r| r.lower() <= polygon && polygon <= r.upper()));
        }

        for bbox in &disjoint {
            let ranges = sfc.ranges(bbox.0, bbox.1, bbox.2, bbox.3, None);
            assert!(!ranges
                .iter()
                .any(|r| r.lower() <= polygon && polygon <= r.upper()));
        }
    }

    #[test]
    fn test_index() {
        let sfc = XZ2SFC::wgs84(12);
        assert_eq!(sfc.index(10.0, 10.0, 12.0, 12.0), 16841390);
        assert_eq!(sfc.index(-180.0, -90.0, -180.0, -90.0), 12);
        assert_eq!(sfc.index(-180.0, -90.0, 0.0, 0.0), 2);
        assert_eq!(sfc.index(10.0, -90.0, 12.0, -89.0), 5599580);
        assert_eq!(sfc.index(79.9, 0.5, 79.9, 0.5), 17236267);
    }

    #[test]
    fn test_ranges() {
        let sfc = XZ2SFC::wgs84(20);

        assert_eq!(sfc.ranges(-0.5, -0.5, 0.5, 0.5, None).len(), 8077);
        assert!(sfc.ranges(-0.5, -0.5, 0.5, 0.5, Some(1000)).len() < 1000);

        assert_eq!(sfc.ranges(55.758, 20.5, 55.759, 21.5, None).len(), 5883);
        assert_eq!(sfc.ranges(-55.758, 20.5, -55.755, 21.5, None).len(), 8070);

        let ranges = sfc.ranges(-55.758, 20.5, -55.755, 21.5, None);

        assert_eq!(ranges.first().map(|r| r.lower()), Some(1));
        assert_eq!(ranges.last().map(|r| r.upper()), Some(847016214083));
    }
}
