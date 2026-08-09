use gungraun::prelude::*;
use spike_numeric_array::{DataArray, Points};
use std::hint::black_box;
use wide::f64x4;

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

/// Control for isolating ADR 0004's storage/dispatch cost from generic Rust-vs-C++ kernel
/// codegen differences: byte-identical kernel body to `Points::bounds()`, but over a bare
/// owned `Vec<f64>` — no `Arc`, no `RwLock`, no enum dispatch. Same by-value ownership (and
/// thus the same in-region `Vec` drop) as `bounds_of_points` above, so that asymmetry cancels
/// out of the comparison between these two benchmarks.
fn bounds_of_slice(xyz: &[f64]) -> Option<[f64; 6]> {
    let mut chunks = xyz.chunks_exact(3);
    let first = chunks.next()?;
    let mut bounds = [first[0], first[0], first[1], first[1], first[2], first[2]];
    for p in chunks {
        if p[0] < bounds[0] {
            bounds[0] = p[0];
        }
        if p[0] > bounds[1] {
            bounds[1] = p[0];
        }
        if p[1] < bounds[2] {
            bounds[2] = p[1];
        }
        if p[1] > bounds[3] {
            bounds[3] = p[1];
        }
        if p[2] < bounds[4] {
            bounds[4] = p[2];
        }
        if p[2] > bounds[5] {
            bounds[5] = p[2];
        }
    }
    Some(bounds)
}

fn setup_coords() -> Vec<f64> {
    generate_coords(NUM_POINTS)
}

#[library_benchmark(setup = setup_coords)]
fn bounds_of_bare_vec(xyz: Vec<f64>) -> Option<[f64; 6]> {
    black_box(bounds_of_slice(&xyz))
}

/// Second, vectorization-friendly restructure of the same kernel, tried after finding upstream
/// rust-lang/rust issues (e.g. #128077, #106539) documenting that `chunks_exact` and branchy
/// if-comparisons are known LLVM autovectorization blockers. Two changes from `bounds_of_slice`:
/// plain indexed access instead of `chunks_exact` (sidesteps #128077), and `f64::min`/`f64::max`
/// (compiles to the `minnum`/`maxnum` LLVM intrinsics, branchless) instead of branchy `if`
/// comparisons. Data volume and by-value ownership are unchanged, so it's directly comparable to
/// `bounds_of_bare_vec` above.
fn bounds_of_slice_indexed_minmax(xyz: &[f64]) -> Option<[f64; 6]> {
    let n = xyz.len() / 3;
    if n == 0 {
        return None;
    }
    let mut bounds = [xyz[0], xyz[0], xyz[1], xyz[1], xyz[2], xyz[2]];
    for i in 1..n {
        let base = i * 3;
        let x = xyz[base];
        let y = xyz[base + 1];
        let z = xyz[base + 2];
        bounds[0] = bounds[0].min(x);
        bounds[1] = bounds[1].max(x);
        bounds[2] = bounds[2].min(y);
        bounds[3] = bounds[3].max(y);
        bounds[4] = bounds[4].min(z);
        bounds[5] = bounds[5].max(z);
    }
    Some(bounds)
}

#[library_benchmark(setup = setup_coords)]
fn bounds_of_bare_vec_indexed_minmax(xyz: Vec<f64>) -> Option<[f64; 6]> {
    black_box(bounds_of_slice_indexed_minmax(&xyz))
}

/// Third restructure, using the `wide` crate's portable SIMD types (stable-Rust alternative to
/// the nightly-only `std::simd`). Data is interleaved AoS (x0,y0,z0,x1,y1,z1,...), so there is no
/// contiguous 4-wide load available per axis; each `f64x4` is built from 4 strided scalar reads
/// (one per point in the group), then `min`/`max` runs as a single SIMD instruction across all 4
/// lanes instead of 4 separate scalar comparisons. A scalar tail handles any points left over
/// when the count isn't a multiple of 4. `min`/`max` (not `fast_min`/`fast_max`) is used to keep
/// NaN handling equivalent to `f64::min`/`f64::max` used in `bounds_of_slice_indexed_minmax`.
fn bounds_of_slice_wide_simd(xyz: &[f64]) -> Option<[f64; 6]> {
    let n = xyz.len() / 3;
    if n == 0 {
        return None;
    }

    let mut min_x = f64x4::splat(xyz[0]);
    let mut max_x = f64x4::splat(xyz[0]);
    let mut min_y = f64x4::splat(xyz[1]);
    let mut max_y = f64x4::splat(xyz[1]);
    let mut min_z = f64x4::splat(xyz[2]);
    let mut max_z = f64x4::splat(xyz[2]);

    let mut i = 1;
    while i + 4 <= n {
        let idx = [i, i + 1, i + 2, i + 3];
        let xs = f64x4::new(idx.map(|j| xyz[j * 3]));
        let ys = f64x4::new(idx.map(|j| xyz[j * 3 + 1]));
        let zs = f64x4::new(idx.map(|j| xyz[j * 3 + 2]));
        min_x = min_x.min(xs);
        max_x = max_x.max(xs);
        min_y = min_y.min(ys);
        max_y = max_y.max(ys);
        min_z = min_z.min(zs);
        max_z = max_z.max(zs);
        i += 4;
    }

    let mut bounds = [
        min_x.to_array().into_iter().fold(f64::INFINITY, f64::min),
        max_x
            .to_array()
            .into_iter()
            .fold(f64::NEG_INFINITY, f64::max),
        min_y.to_array().into_iter().fold(f64::INFINITY, f64::min),
        max_y
            .to_array()
            .into_iter()
            .fold(f64::NEG_INFINITY, f64::max),
        min_z.to_array().into_iter().fold(f64::INFINITY, f64::min),
        max_z
            .to_array()
            .into_iter()
            .fold(f64::NEG_INFINITY, f64::max),
    ];

    for j in i..n {
        let x = xyz[j * 3];
        let y = xyz[j * 3 + 1];
        let z = xyz[j * 3 + 2];
        bounds[0] = bounds[0].min(x);
        bounds[1] = bounds[1].max(x);
        bounds[2] = bounds[2].min(y);
        bounds[3] = bounds[3].max(y);
        bounds[4] = bounds[4].min(z);
        bounds[5] = bounds[5].max(z);
    }

    Some(bounds)
}

#[library_benchmark(setup = setup_coords)]
fn bounds_of_bare_vec_wide_simd(xyz: Vec<f64>) -> Option<[f64; 6]> {
    black_box(bounds_of_slice_wide_simd(&xyz))
}

library_benchmark_group!(
    name = points_bounds_group,
    benchmarks = [
        bounds_of_points,
        bounds_of_bare_vec,
        bounds_of_bare_vec_indexed_minmax,
        bounds_of_bare_vec_wide_simd
    ]
);
main!(library_benchmark_groups = points_bounds_group);

// No #[test]s here: this bench target sets `harness = false` so gungraun's own `main!` runs
// unconditionally, including under `cargo test` — a `#[cfg(test)] mod tests` block in this file
// compiles but its #[test] functions are never actually invoked by any harness. Correctness of
// `bounds_of_slice_wide_simd` against the `bounds_of_slice` scalar reference (n = 0, 1, and every
// residue mod 4 through 1,000,000) was instead checked with a standalone script outside this
// crate; see the SDD ledger for this benchmark task for the check and its output.
