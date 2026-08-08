use gungraun::prelude::*;
use spike_numeric_array::{DataArray, Points};
use std::hint::black_box;

const NUM_POINTS: usize = 1_000_000;

/// Deterministic, integer-derived coordinate generator — see this task's plan brief for why
/// (must be bit-identical with the C++ reference in `cpp/points_bounds.cpp`, and no libm
/// transcendentals since those differ across toolchains).
fn generate_coords(n_points: usize) -> Vec<f64> {
    let mut coords = Vec::with_capacity(n_points * 3);
    for i in 0..(n_points * 3) as u64 {
        let bits = i.wrapping_mul(2654435761);
        coords.push((bits % 100_000) as f64 / 1000.0);
    }
    coords
}

/// Runs outside the measured region — gungraun's `setup` mechanism excludes this from the
/// reported instruction counts.
fn setup_points() -> Points {
    Points::new(DataArray::from_f64(generate_coords(NUM_POINTS))).unwrap()
}

#[library_benchmark(setup = setup_points)]
fn bounds_of_points(points: Points) -> Option<[f64; 6]> {
    black_box(points.bounds())
}

library_benchmark_group!(name = points_bounds_group, benchmarks = bounds_of_points);
main!(library_benchmark_groups = points_bounds_group);
