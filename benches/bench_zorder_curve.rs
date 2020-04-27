#![feature(test)]

use space_time::{RangeComputeHints, SpaceFillingCurves};

extern crate test;

use test::Bencher;

#[bench]
fn test_space_filling_curve_2d_zorder_city_size(b: &mut Bencher) {
    let curve = SpaceFillingCurves::get_point_curve(1024, -180.0, -90.0, 180.0, 90.0);
    let x_min = -174.45869;
    let x_max = -174.12485;
    let y_min = 56.345605;
    let y_max = 56.95869;

    b.iter(|| {
        let _range = curve.ranges(
            x_min,
            y_min,
            x_max,
            y_max,
            &[RangeComputeHints::MaxRecurse(32)],
        );
    })
}

#[bench]
fn test_space_filling_curve_2d_zorder_state_size(b: &mut Bencher) {
    let curve = SpaceFillingCurves::get_point_curve(1024, -180.0, -90.0, 180.0, 90.0);
    let x_min = -93.245;
    let x_max = -88.24849;
    let y_min = 42.01485;
    let y_max = 46.28405;

    b.iter(|| {
        let _range = curve.ranges(
            x_min,
            y_min,
            x_max,
            y_max,
            &[RangeComputeHints::MaxRecurse(32)],
        );
    })
}

#[bench]
fn test_space_filling_curve_2d_zorder_country_size(b: &mut Bencher) {
    let curve = SpaceFillingCurves::get_point_curve(1024, -180.0, -90.0, 180.0, 90.0);
    let x_min = 53.4588044297;
    let x_max = 135.026311477;
    let y_min = 18.197700914;
    let y_max = 73.6753792663;

    b.iter(|| {
        let _range = curve.ranges(
            x_min,
            y_min,
            x_max,
            y_max,
            &[RangeComputeHints::MaxRecurse(32)],
        );
    })
}
