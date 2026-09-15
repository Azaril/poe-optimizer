//! Fixed numerical vectors consumed by Import and the executed pinned source test.
//! Expected values are explicit data, never calculated by a replacement oracle.
pub const SYMMETRIC_HALF_OFFSET: &[(f64, f64)] = &[
    (-3.5, -4.0),
    (-2.5, -3.0),
    (-0.5, -1.0),
    (-0.25, -0.0),
    (-0.0, 0.0),
    (0.0, 0.0),
    (0.25, 0.0),
    (0.5, 1.0),
    (2.5, 3.0),
    (3.5, 4.0),
    (f64::from_bits(0x3fdffffffffffffe), 0.0),
    (f64::from_bits(0x3fdfffffffffffff), 1.0),
    (-f64::from_bits(0x3fdffffffffffffe), -0.0),
    (-f64::from_bits(0x3fdfffffffffffff), -1.0),
    (4_503_599_627_370_495.0, 4_503_599_627_370_495.0),
    (4_503_599_627_370_496.0, 4_503_599_627_370_496.0),
    (4_503_599_627_370_497.0, 4_503_599_627_370_498.0),
    (-4_503_599_627_370_497.0, -4_503_599_627_370_498.0),
    (9_007_199_254_740_991.0, 9_007_199_254_740_992.0),
];
