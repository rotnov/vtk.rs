# Numeric-array storage benchmark (ADR 0004 validation) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the minimal `DataArray`/`Points` slice of ADR 0004's design and run the
ADR-mandated synthetic microbenchmark (Rust vs. equivalent C++, deterministic instruction
counts) to decide whether the storage strategy is validated before it is propagated beyond this
spike. This plan stops at the benchmark — it does not port `vtk-common-core` itself.

**Architecture:** A `DataArray` enum (10 fixed-width numeric variants, `Arc<RwLock<Vec<T>>>`
storage) plus a `Points` wrapper exercising the ADR's "match once per call" dispatch pattern via
a `bounds()` kernel. The kernel is benchmarked with `gungraun` (deterministic instruction/cache
counts via Valgrind/Callgrind) against a byte-for-bit equivalent C++ implementation measured with
raw Callgrind, scoped to the kernel only via `CALLGRIND_TOGGLE_COLLECT`.

**Tech Stack:** Rust (`edition = "2024"`), `gungraun` 0.19.4 for the Rust-side benchmark,
Valgrind/Callgrind directly for the C++ reference, GitHub Actions (`workflow_dispatch`) to run
both under identical conditions.

## Global Constraints

- **This is a validation spike, not the `vtk-common-core` port.** It lives in a new workspace
  member `rust/spike-numeric-array/`, deliberately **not** added to `WORKSPACE_CRATES` in
  `rust/xtask/src/main.rs`. Verified empirically: `crate_code_flags()` (the function that feeds
  `check_parity`) only walks that hardcoded list and joins `rust/crates/<name>/src` — a crate
  outside `rust/crates/` and absent from the list is invisible to `cargo xtask ledger-check`'s
  four assertions. Do not add this crate to `WORKSPACE_CRATES`, to `docs/test-mapping.csv`, or to
  the `cargo-check-wasm32` job's `-p` list in `.github/workflows/rust-checks.yml` — doing any of
  those would incorrectly represent this spike as a ported module.
- **No coverage-gate task in this plan.** `docs/decisions/0001-test-coverage-metric.md` mandates
  `cargo llvm-cov --workspace --all-features --fail-under-lines 100 --fail-under-functions 100`
  starting "the same PR that adds the first crate with an actually-executing test." Measured
  directly during the prior `ledger-check` plan's final review: the workspace is at
  **79.58% lines / 85.39% functions today**, entirely from pre-existing `xtask` code unrelated to
  this plan (tracked as [issue #54](https://github.com/rotnov/vtk.rs/issues/54), a policy
  question about whether the gate should even apply to `xtask`). Wiring `--fail-under-lines 100`
  now would fail immediately on code this plan does not touch. This plan's own new tests must
  still pass `cargo test --workspace --all-features`, `cargo clippy --workspace --all-targets
  --all-features -- -D warnings`, and `cargo fmt --all --check` — those three are "Live today"
  per `AGENTS.md` § Required checks and apply to every workspace member automatically.
- **`spike-numeric-array`'s dependency-graph is exempt from AGENTS.md's module-DEPENDS-mirroring
  policy**, for the same reason `xtask`'s `csv` dependency was exempted during the `ledger-check`
  plan: it is not a `vtk-<module>` port crate, so there is no VTK module DEPENDS list for it to
  mirror. Its only dependency is `gungraun` (dev-only, benchmark harness).
- **AGENTS.md § Rust workspace conventions, "No 1:1 class-for-class translation":** *"Prefer:
  `enum` + traits for polymorphism where VTK uses a small closed set of subclasses... generics
  where VTK uses `vtkTemplateMacro`-style dispatch."* `DataArray` is exactly this: one enum, not
  a generic parameter, matching ADR 0004's decision.
- **ADR 0004, "Dispatch: match once per call, not once per element":** *"a `match` or lock
  acquisition inside the inner loop would reintroduce per-element overhead the C++ original never
  pays, undermining the entire performance rationale for the port before it is even tested."*
  `Points::bounds()` must acquire the `RwLock` exactly once per call (at the top of the method),
  never per point.
- **ADR 0004, storage:** each `DataArray` variant wraps `Arc<RwLock<Vec<T>>>`; `Clone` must be an
  O(1) `Arc` pointer copy, not a deep copy, and two clones must observe each other's mutations.
- **ADR 0004, scope note this plan adds:** real `vtkPoints` accepts any `vtkDataArray` concrete
  type via `SetDataType`. This spike's `Points` fixes the backing type to `F64` only, because the
  benchmark measures a double-precision kernel against equivalent C++ `double` data — supporting
  the other 9 `DataArray` variants in `Points` is out of scope here and deferred to the real
  `vtkPoints` port task under issue #45.
- **ADR 0004 names `gungraun`, not `iai-callgrind`**, as of commit `c185f55d85` on this branch
  (`iai-callgrind` rebranded and is frozen at v0.16.1; `gungraun` is the actively maintained
  successor, v0.19.4). Use `gungraun = "0.19.4"` verbatim in `Cargo.toml`.
- **Benchmark methodology — kernel-only measurement, decided during planning to avoid a
  meaningless ratio:** data generation must happen outside the measured region on both sides
  (`gungraun`'s `setup` parameter on the Rust side; `CALLGRIND_TOGGLE_COLLECT` bracketing on the
  C++ side, run with `--collect-atstart=no`). Both sides must generate **identical** coordinate
  data via the same deterministic, integer-derived formula (given verbatim in Task 3 and Task 4)
  — no `sin`/`cos`, since libm implementations differ across toolchains and floating-point
  transcendental results would not be bit-identical between Rust and C++.
- Commands in this plan run from `rust/` unless a step says otherwise, matching this repo's
  existing convention (`AGENTS.md` § Commands).

---

## File Structure

- `rust/Cargo.toml` — add `"spike-numeric-array"` to `[workspace.members]`; add `[profile.bench]
  debug = true` (required by `gungraun`/Valgrind to resolve debug symbols).
- `rust/spike-numeric-array/Cargo.toml` — new, standalone (no `.workspace = true` inheritance —
  this crate deliberately opts out of the `vtk-*` package conventions since it isn't one).
- `rust/spike-numeric-array/README.md` — new: states the spike's purpose, scope, and how to run
  the benchmark manually.
- `rust/spike-numeric-array/.gitignore` — new: ignores the compiled C++ binary and Callgrind
  output files under `cpp/`.
- `rust/spike-numeric-array/src/lib.rs` — new: re-exports `array` and `points` modules.
- `rust/spike-numeric-array/src/array.rs` — new: `DataArray` enum + tests.
- `rust/spike-numeric-array/src/points.rs` — new: `Points` wrapper + `bounds()` + tests.
- `rust/spike-numeric-array/benches/points_bounds.rs` — new: `gungraun` benchmark harness.
- `rust/spike-numeric-array/cpp/points_bounds.cpp` — new: C++ reference implementation.
- `rust/spike-numeric-array/cpp/run_benchmark.sh` — new: builds and runs the C++ reference under
  Callgrind.
- `.github/workflows/benchmark-validation.yml` — new: manually-triggered (`workflow_dispatch`)
  workflow running both benchmarks and publishing results to the job summary.

---

### Task 1: `spike-numeric-array` crate skeleton + `DataArray` enum

**Files:**
- Create: `rust/spike-numeric-array/Cargo.toml`
- Create: `rust/spike-numeric-array/src/lib.rs`
- Create: `rust/spike-numeric-array/src/array.rs`
- Create: `rust/spike-numeric-array/README.md`
- Modify: `rust/Cargo.toml` (add workspace member)

**Interfaces:**
- Produces: `pub enum DataArray` with variants `F32(Arc<RwLock<Vec<f32>>>)`,
  `F64(Arc<RwLock<Vec<f64>>>)`, `I8`, `U8`, `I16`, `U16`, `I32`, `U32`, `I64`, `U64` (same shape,
  one `Arc<RwLock<Vec<T>>>` each); `DataArray::from_f32(Vec<f32>) -> Self` ... `from_u64(Vec<u64>)
  -> Self` (10 constructors, one per variant); `DataArray::len(&self) -> usize`;
  `DataArray::is_empty(&self) -> bool`. `DataArray` derives `Clone`.

- [ ] **Step 1: Add the workspace member**

Edit `rust/Cargo.toml`. Find the `members = [...]` list and add `"spike-numeric-array"` to it
(order doesn't matter, but keep the list alphabetized if it already is). Then add this section
anywhere at the top level of the file, after `[workspace]`:

```toml
[profile.bench]
debug = true
```

- [ ] **Step 2: Create the crate manifest**

Create `rust/spike-numeric-array/Cargo.toml`:

```toml
[package]
name = "spike-numeric-array"
version = "0.0.0"
edition = "2024"
publish = false
description = "Validation spike for docs/decisions/0004-numeric-array-storage.md — not part of the vtk-common-core port."

[dependencies]
```

- [ ] **Step 3: Write the failing test for `DataArray::len`/`is_empty`**

Create `rust/spike-numeric-array/src/array.rs` with only the test module first:

```rust
//! `DataArray`: a runtime-typed numeric buffer, one variant per fixed-width numeric type.
//!
//! Validation spike for docs/decisions/0004-numeric-array-storage.md — not part of the
//! `vtk-common-core` port. See `rust/spike-numeric-array/README.md`.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_variant_reports_its_length() {
        let cases: Vec<(DataArray, usize)> = vec![
            (DataArray::from_f32(vec![1.0, 2.0, 3.0]), 3),
            (DataArray::from_f64(vec![1.0, 2.0]), 2),
            (DataArray::from_i8(vec![1]), 1),
            (DataArray::from_u8(vec![1, 2, 3, 4]), 4),
            (DataArray::from_i16(vec![]), 0),
            (DataArray::from_u16(vec![1, 2]), 2),
            (DataArray::from_i32(vec![1, 2, 3]), 3),
            (DataArray::from_u32(vec![1]), 1),
            (DataArray::from_i64(vec![1, 2, 3, 4, 5]), 5),
            (DataArray::from_u64(vec![1, 2]), 2),
        ];
        for (array, expected_len) in cases {
            assert_eq!(array.len(), expected_len);
            assert_eq!(array.is_empty(), expected_len == 0);
        }
    }

    #[test]
    fn clone_shares_identity_and_mutations() {
        let original = DataArray::from_f64(vec![1.0, 2.0, 3.0]);
        let alias = original.clone();

        if let DataArray::F64(buf) = &original {
            buf.write().unwrap()[0] = 99.0;
        } else {
            panic!("expected F64 variant");
        }

        if let DataArray::F64(buf) = &alias {
            assert_eq!(buf.read().unwrap()[0], 99.0);
        } else {
            panic!("expected F64 variant");
        }
    }
}
```

- [ ] **Step 4: Run the test to verify it fails**

Run: `cargo test -p spike-numeric-array` (from `rust/`)
Expected: compile error — `DataArray` does not exist yet.

- [ ] **Step 5: Implement `DataArray`**

Add above the `#[cfg(test)]` module in `rust/spike-numeric-array/src/array.rs`:

```rust
use std::sync::{Arc, RwLock};

/// One concrete type for "an array of numeric type unknown until runtime" — see ADR 0004.
#[derive(Clone)]
pub enum DataArray {
    F32(Arc<RwLock<Vec<f32>>>),
    F64(Arc<RwLock<Vec<f64>>>),
    I8(Arc<RwLock<Vec<i8>>>),
    U8(Arc<RwLock<Vec<u8>>>),
    I16(Arc<RwLock<Vec<i16>>>),
    U16(Arc<RwLock<Vec<u16>>>),
    I32(Arc<RwLock<Vec<i32>>>),
    U32(Arc<RwLock<Vec<u32>>>),
    I64(Arc<RwLock<Vec<i64>>>),
    U64(Arc<RwLock<Vec<u64>>>),
}

impl DataArray {
    pub fn from_f32(values: Vec<f32>) -> Self {
        DataArray::F32(Arc::new(RwLock::new(values)))
    }
    pub fn from_f64(values: Vec<f64>) -> Self {
        DataArray::F64(Arc::new(RwLock::new(values)))
    }
    pub fn from_i8(values: Vec<i8>) -> Self {
        DataArray::I8(Arc::new(RwLock::new(values)))
    }
    pub fn from_u8(values: Vec<u8>) -> Self {
        DataArray::U8(Arc::new(RwLock::new(values)))
    }
    pub fn from_i16(values: Vec<i16>) -> Self {
        DataArray::I16(Arc::new(RwLock::new(values)))
    }
    pub fn from_u16(values: Vec<u16>) -> Self {
        DataArray::U16(Arc::new(RwLock::new(values)))
    }
    pub fn from_i32(values: Vec<i32>) -> Self {
        DataArray::I32(Arc::new(RwLock::new(values)))
    }
    pub fn from_u32(values: Vec<u32>) -> Self {
        DataArray::U32(Arc::new(RwLock::new(values)))
    }
    pub fn from_i64(values: Vec<i64>) -> Self {
        DataArray::I64(Arc::new(RwLock::new(values)))
    }
    pub fn from_u64(values: Vec<u64>) -> Self {
        DataArray::U64(Arc::new(RwLock::new(values)))
    }

    /// Total element count across the flat buffer (not tuple count). Dispatched once per call —
    /// see ADR 0004's "match once per call, not once per element".
    pub fn len(&self) -> usize {
        match self {
            DataArray::F32(b) => b.read().unwrap().len(),
            DataArray::F64(b) => b.read().unwrap().len(),
            DataArray::I8(b) => b.read().unwrap().len(),
            DataArray::U8(b) => b.read().unwrap().len(),
            DataArray::I16(b) => b.read().unwrap().len(),
            DataArray::U16(b) => b.read().unwrap().len(),
            DataArray::I32(b) => b.read().unwrap().len(),
            DataArray::U32(b) => b.read().unwrap().len(),
            DataArray::I64(b) => b.read().unwrap().len(),
            DataArray::U64(b) => b.read().unwrap().len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
```

- [ ] **Step 6: Run the tests to verify they pass**

Run: `cargo test -p spike-numeric-array` (from `rust/`)
Expected: 2 passed (`each_variant_reports_its_length`, `clone_shares_identity_and_mutations`).

- [ ] **Step 7: Create `lib.rs`**

Create `rust/spike-numeric-array/src/lib.rs`:

```rust
//! Validation spike for docs/decisions/0004-numeric-array-storage.md.
//!
//! Not part of the `vtk-common-core` port — see `README.md` in this crate's root. Exists only
//! to run the `DataArray`/`Points` design through a real dispatch-and-bounds workload and
//! benchmark it against equivalent C++, per the ADR's "Validation is required" clause.

pub mod array;

pub use array::DataArray;
```

- [ ] **Step 8: Create the README**

Create `rust/spike-numeric-array/README.md`:

```markdown
# spike-numeric-array

Validation spike for [ADR 0004](../../docs/decisions/0004-numeric-array-storage.md) — **not**
part of the `vtk-common-core` port, and deliberately excluded from `cargo xtask ledger-check`
(not listed in `WORKSPACE_CRATES` in `rust/xtask/src/main.rs`) and from the `cargo-check-wasm32`
CI job.

## What this is

A minimal `DataArray` enum (one variant per fixed-width numeric type, `Arc<RwLock<Vec<T>>>`
storage) and a `Points` wrapper over it, built only to run ADR 0004's storage/dispatch design
through a representative kernel (`Points::bounds()`) and benchmark it against equivalent C++,
measured with deterministic instruction counts (Valgrind/Callgrind via `gungraun` on the Rust
side, raw Callgrind on the C++ side).

## Running the benchmark locally

Requires Valgrind (`apt-get install valgrind` on Debian/Ubuntu) and a `gungraun-runner` matching
the `gungraun` version pinned in `Cargo.toml`:

```bash
cargo install --version 0.19.4 gungraun-runner
cd rust
cargo bench -p spike-numeric-array --bench points_bounds
```

For the C++ reference implementation:

```bash
cd rust/spike-numeric-array/cpp
./run_benchmark.sh
```

## What happens to this crate after the benchmark

This crate is scaffolding for one decision (whether ADR 0004's design holds up against
equivalent C++), not a foundation to build on. Once the benchmark result is reported, it should
either be deleted or, if a real need arises, promoted into `vtk-common-core` proper (adding it to
`WORKSPACE_CRATES`, `docs/test-mapping.csv`, and the wasm32 check job) — not left in this
in-between state indefinitely.
```

- [ ] **Step 9: Run the full workspace suite**

Run (from `rust/`): `cargo build --workspace && cargo test --workspace --all-features && cargo
clippy --workspace --all-targets --all-features -- -D warnings && cargo fmt --all --check`
Expected: all green, including the two new tests in `spike-numeric-array`.

- [ ] **Step 10: Commit**

```bash
git add rust/Cargo.toml rust/Cargo.lock rust/spike-numeric-array/Cargo.toml \
  rust/spike-numeric-array/src/lib.rs rust/spike-numeric-array/src/array.rs \
  rust/spike-numeric-array/README.md
git commit -m "spike-numeric-array: DataArray enum (ADR 0004 validation)"
```

---

### Task 2: `Points` wrapper + `bounds()` kernel

**Files:**
- Create: `rust/spike-numeric-array/src/points.rs`
- Modify: `rust/spike-numeric-array/src/lib.rs` (add `pub mod points;` and re-exports)

**Interfaces:**
- Consumes: `spike_numeric_array::array::DataArray` (specifically the `DataArray::F64` variant;
  from Task 1), constructed via `DataArray::from_f64(Vec<f64>)`.
- Produces: `pub enum PointsError { NotDivisibleByThree, RequiresF64 }` (derives `Debug`,
  `PartialEq`); `pub struct Points`; `Points::new(data: DataArray) -> Result<Points,
  PointsError>`; `Points::bounds(&self) -> Option<[f64; 6]>` (layout: `[xmin, xmax, ymin, ymax,
  zmin, zmax]`, `None` for zero points). These two names (`Points`, `PointsError`) are what
  Task 3's benchmark harness imports.

- [ ] **Step 1: Write the failing tests**

Create `rust/spike-numeric-array/src/points.rs`:

```rust
//! `Points`: minimal `vtkPoints`-equivalent wrapper over an `F64` `DataArray`, used only to
//! exercise ADR 0004's storage/dispatch design end-to-end for the validation benchmark.
//!
//! Scope note: real `vtkPoints` accepts any of `vtkDataArray`'s concrete types via
//! `SetDataType`; this spike fixes it to `F64` because the benchmark measures a
//! double-precision bounds kernel against equivalent C++ `double` data. Supporting the other
//! nine `DataArray` variants is deferred to the real `vtkPoints` port task (issue #45).

use crate::array::DataArray;
use std::sync::{Arc, RwLock};

#[derive(Debug, PartialEq)]
pub enum PointsError {
    /// `DataArray` length is not a multiple of 3 (points are 3-component tuples).
    NotDivisibleByThree,
    /// `Points` requires an `F64`-backed `DataArray` — see the module scope note.
    RequiresF64,
}

pub struct Points {
    xyz: Arc<RwLock<Vec<f64>>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_non_f64_data_arrays() {
        let data = DataArray::from_i32(vec![1, 2, 3]);
        assert_eq!(Points::new(data), Err(PointsError::RequiresF64));
    }

    #[test]
    fn rejects_lengths_not_divisible_by_three() {
        let data = DataArray::from_f64(vec![1.0, 2.0]);
        assert_eq!(Points::new(data), Err(PointsError::NotDivisibleByThree));
    }

    #[test]
    fn bounds_of_empty_points_is_none() {
        let points = Points::new(DataArray::from_f64(vec![])).unwrap();
        assert_eq!(points.bounds(), None);
    }

    #[test]
    fn bounds_of_single_point_is_degenerate() {
        let points = Points::new(DataArray::from_f64(vec![1.0, 2.0, 3.0])).unwrap();
        assert_eq!(points.bounds(), Some([1.0, 1.0, 2.0, 2.0, 3.0, 3.0]));
    }

    #[test]
    fn bounds_of_several_points() {
        let points = Points::new(DataArray::from_f64(vec![
            0.0, 0.0, 0.0, -1.0, 5.0, 2.0, 3.0, -4.0, 2.0,
        ]))
        .unwrap();
        assert_eq!(points.bounds(), Some([-1.0, 3.0, -4.0, 5.0, 0.0, 2.0]));
    }
}
```

Note: `Points` has no methods yet (`new`/`bounds` not defined) — this is deliberate, Step 3 below
adds them. Do not add any derive to `Points` itself: `RwLock` does not implement `PartialEq`, so
`#[derive(PartialEq)]` on `Points` would not compile. Only `PointsError` needs `Debug +
PartialEq` (already on it above) — the tests compare `Points::new(...)`'s `Err(...)` case against
`PointsError` values, and call `.unwrap()` on the `Ok(...)` case, neither of which needs any
derive on `Points` itself.

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p spike-numeric-array` (from `rust/`)
Expected: compile error — `Points::new` and `Points::bounds` don't exist yet.

- [ ] **Step 3: Implement `Points::new` and `Points::bounds`**

Add to `rust/spike-numeric-array/src/points.rs`, after the `Points` struct definition and before
the `#[cfg(test)]` module:

```rust
impl Points {
    pub fn new(data: DataArray) -> Result<Self, PointsError> {
        match data {
            DataArray::F64(buf) => {
                if buf.read().unwrap().len() % 3 != 0 {
                    return Err(PointsError::NotDivisibleByThree);
                }
                Ok(Points { xyz: buf })
            }
            _ => Err(PointsError::RequiresF64),
        }
    }

    /// `[xmin, xmax, ymin, ymax, zmin, zmax]`, matching `vtkPoints::GetBounds()`'s layout.
    /// `None` for an empty point set (real `vtkPoints::GetBounds()` on 0 points leaves
    /// `VTK_DOUBLE_MAX`/`-VTK_DOUBLE_MAX` sentinels instead — surfaced here as `None`).
    ///
    /// Acquires the lock exactly once — see ADR 0004's "match once per call, not once per
    /// element" (this is the per-call dispatch, even though there's only one variant to match
    /// here; the lock acquisition itself must not repeat per point).
    pub fn bounds(&self) -> Option<[f64; 6]> {
        let guard = self.xyz.read().unwrap();
        let mut chunks = guard.chunks_exact(3);
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
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p spike-numeric-array` (from `rust/`)
Expected: 7 passed (2 from Task 1's `array.rs` + 5 here).

- [ ] **Step 5: Wire `points` into `lib.rs`**

Edit `rust/spike-numeric-array/src/lib.rs` to:

```rust
//! Validation spike for docs/decisions/0004-numeric-array-storage.md.
//!
//! Not part of the `vtk-common-core` port — see `README.md` in this crate's root. Exists only
//! to run the `DataArray`/`Points` design through a real dispatch-and-bounds workload and
//! benchmark it against equivalent C++, per the ADR's "Validation is required" clause.

pub mod array;
pub mod points;

pub use array::DataArray;
pub use points::{Points, PointsError};
```

- [ ] **Step 6: Run the full workspace suite**

Run (from `rust/`): `cargo build --workspace && cargo test --workspace --all-features && cargo
clippy --workspace --all-targets --all-features -- -D warnings && cargo fmt --all --check`
Expected: all green, 7 tests in `spike-numeric-array`.

- [ ] **Step 7: Commit**

```bash
git add rust/spike-numeric-array/src/points.rs rust/spike-numeric-array/src/lib.rs
git commit -m "spike-numeric-array: Points wrapper and bounds() kernel"
```

---

### Task 3: `gungraun` benchmark harness (Rust side)

**Files:**
- Modify: `rust/spike-numeric-array/Cargo.toml` (add `gungraun` dev-dependency + `[[bench]]`)
- Create: `rust/spike-numeric-array/benches/points_bounds.rs`

**Interfaces:**
- Consumes: `spike_numeric_array::{DataArray, Points}` (from Tasks 1-2).
- Produces: the exact data-generation formula that Task 4's C++ file must reproduce bit-for-bit:
  for flat index `i` in `0..(n_points * 3)` as `u64`, `coords[i] = ((i.wrapping_mul(2654435761))
  % 100_000) as f64 / 1000.0`. `n_points = 1_000_000`.

- [ ] **Step 1: Add the `gungraun` dev-dependency and bench target**

Edit `rust/spike-numeric-array/Cargo.toml` to add:

```toml
[dev-dependencies]
gungraun = "0.19.4"

[[bench]]
name = "points_bounds"
harness = false
```

- [ ] **Step 2: Write the benchmark harness**

Create `rust/spike-numeric-array/benches/points_bounds.rs`:

```rust
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
```

- [ ] **Step 3: Install the matching `gungraun-runner` (local verification only — CI installs it
  via `gungraun/setup-gungraun@v1` in Task 5)**

Run: `cargo install --version 0.19.4 gungraun-runner`
(Requires Valgrind installed locally — `apt-get install valgrind` / `brew install valgrind` on
Linux. If Valgrind is unavailable in this environment, skip local execution here — the CI job in
Task 5 is the environment this benchmark is meant to run in, and Task 6's controller-executed
step runs it there.)

- [ ] **Step 4: Run the benchmark and verify it produces output**

Run (from `rust/`): `cargo bench -p spike-numeric-array --bench points_bounds`
Expected: a `gungraun` report block for `bounds_of_points` showing Instructions/L1 Hits/LL
Hits/RAM Hits/Total read+write/Estimated Cycles. If the compiler rejects the exact
`#[library_benchmark(setup = ...)]` sub-case syntax above, consult `gungraun`'s installed docs
(`docs.rs/gungraun`) for the current attribute form — the requirement is fixed (one benchmark
function, data generated in `setup`, not in the measured body), the exact macro spelling is not.
If Valgrind isn't available locally, note that in the task report as `DONE_WITH_CONCERNS` rather
than blocking — Task 5/6 verify it for real in CI.

- [ ] **Step 5: Run the full workspace suite (build only — `cargo test --workspace` does not run
  `harness = false` benches, so this doesn't change Task 2's test count)**

Run (from `rust/`): `cargo build --workspace --all-targets && cargo clippy --workspace
--all-targets --all-features -- -D warnings && cargo fmt --all --check`
Expected: all green, including the new `benches/points_bounds.rs` target.

- [ ] **Step 6: Commit**

```bash
git add rust/spike-numeric-array/Cargo.toml rust/Cargo.lock \
  rust/spike-numeric-array/benches/points_bounds.rs
git commit -m "spike-numeric-array: gungraun benchmark harness for Points::bounds"
```

(There is no per-member `Cargo.lock` — this is a workspace member, so the dependency update
lands in the single `rust/Cargo.lock` at the workspace root, same as Task 1's Step 10.)

---

### Task 4: C++ reference implementation

**Files:**
- Create: `rust/spike-numeric-array/cpp/points_bounds.cpp`
- Create: `rust/spike-numeric-array/cpp/run_benchmark.sh`
- Create: `rust/spike-numeric-array/.gitignore`

**Interfaces:**
- Consumes: the same data-generation formula as Task 3 — for flat index `i` in `0..(n_points *
  3)` as `uint64_t`, `coords[i] = static_cast<double>((i * 2654435761ULL) % 100000ULL) / 1000.0`,
  `n_points = 1'000'000`. This must produce bit-identical values to Task 3's Rust generator (both
  are pure unsigned 64-bit integer arithmetic followed by one `double` division — no
  floating-point operation whose result depends on toolchain/libm).

- [ ] **Step 1: Write the C++ reference implementation**

Create `rust/spike-numeric-array/cpp/points_bounds.cpp`:

```cpp
#include <valgrind/callgrind.h>

#include <cstdint>
#include <cstdio>
#include <optional>
#include <vector>

namespace {

std::vector<double> generate_coords(std::size_t n_points) {
  std::vector<double> coords;
  coords.reserve(n_points * 3);
  for (std::uint64_t i = 0; i < static_cast<std::uint64_t>(n_points * 3); ++i) {
    std::uint64_t bits = i * 2654435761ULL;
    coords.push_back(static_cast<double>(bits % 100000ULL) / 1000.0);
  }
  return coords;
}

struct Bounds {
  double values[6];
};

// Equivalent of Points::bounds() in rust/spike-numeric-array/src/points.rs — acquires nothing
// (no lock in the C++ reference; VTK's own vtkPoints::GetBounds() doesn't lock either), matches
// the Rust kernel's single pass over the flat xyz buffer.
std::optional<Bounds> bounds_of_points(const std::vector<double>& xyz) {
  if (xyz.empty() || xyz.size() % 3 != 0) {
    return std::nullopt;
  }
  Bounds b{{xyz[0], xyz[0], xyz[1], xyz[1], xyz[2], xyz[2]}};
  for (std::size_t i = 3; i < xyz.size(); i += 3) {
    double x = xyz[i];
    double y = xyz[i + 1];
    double z = xyz[i + 2];
    if (x < b.values[0]) b.values[0] = x;
    if (x > b.values[1]) b.values[1] = x;
    if (y < b.values[2]) b.values[2] = y;
    if (y > b.values[3]) b.values[3] = y;
    if (z < b.values[4]) b.values[4] = z;
    if (z > b.values[5]) b.values[5] = z;
  }
  return b;
}

}  // namespace

int main() {
  constexpr std::size_t kNumPoints = 1'000'000;
  std::vector<double> coords = generate_coords(kNumPoints);

  CALLGRIND_TOGGLE_COLLECT;
  std::optional<Bounds> result = bounds_of_points(coords);
  CALLGRIND_TOGGLE_COLLECT;

  if (result) {
    std::printf("bounds: %f %f %f %f %f %f\n", result->values[0], result->values[1],
                result->values[2], result->values[3], result->values[4], result->values[5]);
  }
  return 0;
}
```

- [ ] **Step 2: Write the build/run script**

Create `rust/spike-numeric-array/cpp/run_benchmark.sh`:

```bash
#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"

g++ -O2 -g -std=c++17 points_bounds.cpp -o points_bounds

# --collect-atstart=no: nothing is counted until CALLGRIND_TOGGLE_COLLECT turns collection on
# inside main() — so the printed "summary:" line reflects only the bounds_of_points() call, not
# process startup or generate_coords().
valgrind --tool=callgrind --collect-atstart=no --callgrind-out-file=callgrind.out \
  ./points_bounds

echo "--- full annotated output ---"
callgrind_annotate callgrind.out

echo "--- summary line (matches the 'events:' header line's column order) ---"
grep '^events:' callgrind.out
grep '^summary:' callgrind.out
```

Make it executable: `chmod +x rust/spike-numeric-array/cpp/run_benchmark.sh`

- [ ] **Step 3: Add the `.gitignore` for build artifacts**

Create `rust/spike-numeric-array/.gitignore`:

```
/cpp/points_bounds
/cpp/callgrind.out
```

- [ ] **Step 4: Verify locally if Valgrind and g++ are available — and prove the toggle actually
  scopes the measurement**

Run: `./rust/spike-numeric-array/cpp/run_benchmark.sh`
Expected: compiles, runs, prints a `bounds:` line followed by the annotated Callgrind output and
a `summary:` line.

`CALLGRIND_TOGGLE_COLLECT` toggles collection state; combined with `--collect-atstart=no` the
first call turns collection on and the second turns it off, so `summary:` should reflect only the
bracketed `bounds_of_points(coords)` call. This must be verified empirically, not assumed — a
misconfigured toggle (wrong Valgrind version, macro not taking effect) would silently report a
whole-program total instead, which would then also include `generate_coords()` and invalidate the
whole comparison. Do this A/B check:

1. Note the `summary:` value from the run above (call it `S1`).
2. Temporarily comment out both `CALLGRIND_TOGGLE_COLLECT;` lines in `points_bounds.cpp` and
   change `--collect-atstart=no` to `--collect-atstart=yes` in `run_benchmark.sh`. Rebuild and
   rerun: `./run_benchmark.sh`, note the new `summary:` value (`S2`).
3. Revert both temporary changes (`git checkout -- points_bounds.cpp run_benchmark.sh`).
4. Compare: `S2` (whole-program: generation + kernel) must be **meaningfully larger** than `S1`
   (kernel only) — generating 3,000,000 values is not free. If `S1` and `S2` are within a few
   percent of each other, the toggle is not scoping anything and `summary:` cannot be trusted as
   the kernel-only number — in that case, use `callgrind_annotate callgrind.out`'s per-function
   row for `bounds_of_points` instead (it attributes cost by function regardless of toggle state)
   and record this finding in the task report so Task 6 knows to read `callgrind_annotate`'s
   per-function table rather than the `summary:` line.

If Valgrind/g++ aren't available in this environment at all, note that in the task report as
`DONE_WITH_CONCERNS` — Task 5/6 run this for real in CI, same as Task 3's local-run step.

- [ ] **Step 5: Run the full workspace suite (this task touches no Rust source, so this just
  confirms nothing broke)**

Run (from `rust/`): `cargo build --workspace && cargo test --workspace --all-features && cargo
clippy --workspace --all-targets --all-features -- -D warnings && cargo fmt --all --check`
Expected: all green, unchanged from Task 3.

- [ ] **Step 6: Commit**

```bash
git add rust/spike-numeric-array/cpp/points_bounds.cpp \
  rust/spike-numeric-array/cpp/run_benchmark.sh rust/spike-numeric-array/.gitignore
git commit -m "spike-numeric-array: C++ reference implementation for the bounds benchmark"
```

---

### Task 5: CI workflow to run both benchmarks

**Files:**
- Create: `.github/workflows/benchmark-validation.yml`

**Interfaces:**
- Consumes: `cargo bench -p spike-numeric-array --bench points_bounds` (Task 3's exact bench
  target name) and `rust/spike-numeric-array/cpp/run_benchmark.sh` (Task 4's exact script path).

- [ ] **Step 1: Write the workflow**

This is deliberately **not** added to the existing `rust-checks.yml` (which triggers on every
push/PR) — Valgrind-instrumented runs are slower and this is a one-off validation, not a
continuous regression gate. It gets its own manually-triggered workflow instead.

`gungraun/setup-gungraun@v1` has not been verified to actually exist/resolve on the Marketplace —
the workflow below treats it as best-effort (`continue-on-error: true`) and falls back to
installing `gungraun-runner` manually via `cargo-binstall` plus `apt-get install valgrind` if the
action step fails. If both the action and the fallback fail, that's real information (not a
process bug) — report it in Task 6 rather than debugging CI further.

Create `.github/workflows/benchmark-validation.yml`:

```yaml
name: benchmark-validation

on:
  workflow_dispatch:

jobs:
  points-bounds:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - uses: dtolnay/rust-toolchain@stable

      - uses: Swatinem/rust-cache@v2
        with:
          workspaces: rust

      - uses: gungraun/setup-gungraun@v1
        continue-on-error: true
        id: setup-gungraun

      - name: Install gungraun-runner manually if the action failed
        if: steps.setup-gungraun.outcome == 'failure'
        run: |
          curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash
          cargo binstall --no-confirm gungraun-runner@0.19.4
          sudo apt-get update && sudo apt-get install -y valgrind

      - name: Install g++ for the C++ reference implementation
        run: sudo apt-get update && sudo apt-get install -y g++

      - name: Run Rust (gungraun) benchmark
        working-directory: rust
        run: cargo bench -p spike-numeric-array --bench points_bounds | tee gungraun-output.txt

      - name: Run C++ reference benchmark
        working-directory: rust/spike-numeric-array/cpp
        run: |
          chmod +x run_benchmark.sh
          ./run_benchmark.sh | tee cpp-output.txt

      - name: Publish results to job summary
        run: |
          {
            echo "## Numeric-array storage benchmark (ADR 0004 validation)"
            echo ""
            echo "### Rust (gungraun)"
            echo '```'
            cat rust/gungraun-output.txt
            echo '```'
            echo ""
            echo "### C++ reference (raw Callgrind)"
            echo '```'
            cat rust/spike-numeric-array/cpp/cpp-output.txt
            echo '```'
          } >> "$GITHUB_STEP_SUMMARY"
```

- [ ] **Step 2: Validate the workflow YAML syntax**

Run: `python3 -c "import yaml; yaml.safe_load(open('.github/workflows/benchmark-validation.yml'))"`
Expected: no output (parses cleanly). If `pyyaml` isn't installed, use `uv run --with pyyaml
python3 -c "..."` instead — this repo uses `uv` for new Python tooling, not raw `pip`.

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/benchmark-validation.yml
git commit -m "ci: add manually-triggered benchmark-validation workflow (ADR 0004)"
```

---

### Task 6 (controller-executed, not dispatched to a subagent): run the benchmark and report the result

This step is executed directly by the controller after Task 5 lands (merged to the branch, not
necessarily to `master` — `workflow_dispatch` works from any branch via `--ref`), per the user's
explicit instruction to actually run the benchmark, not just plan it.

- [ ] **Step 1:** Push the branch: `git push -u origin <branch-name>`
- [ ] **Step 2:** Trigger the workflow: `gh workflow run benchmark-validation.yml --ref
  <branch-name>`
- [ ] **Step 3:** Poll for the run and wait for completion: `gh run list --workflow
  benchmark-validation.yml --branch <branch-name> --limit 1` to get the run ID, then `gh run
  watch <run-id>`
- [ ] **Step 4:** Fetch the job summary: `gh run view <run-id> --log` (or `gh api
  repos/rotnov/vtk.rs/actions/runs/<run-id>/jobs` for structured job data) to retrieve both the
  `gungraun` Instructions/Estimated-Cycles numbers and the C++ Callgrind `summary:` line.
- [ ] **Step 5:** Normalize both sides to instructions-per-point before comparing: divide the
  Rust `gungraun` instruction count and the C++ Callgrind kernel-only count (the `summary:` value
  validated in Task 4 Step 4 — or the `callgrind_annotate` per-function row for
  `bounds_of_points`, if Task 4's A/B check found `summary:` untrustworthy) each by 1,000,000.
  This matters because `Points::bounds()` acquires an `RwLock` read guard once per call and the
  C++ reference acquires nothing — a fixed, one-time cost that is real but should not be confused
  with the per-element kernel cost ADR 0004 actually asks about. At 1,000,000 points the lock's
  fixed cost is amortized to near-zero per point, so comparing per-point instruction counts
  isolates the kernel/dispatch cost the ADR is actually about, rather than the lock.
- [ ] **Step 6:** Report the actual numbers to the user, without spin, per the ralph-loop's
  CRITICAL RULE — this is a real go/no-go signal for the whole port project, per the user's own
  framing ("если бенчмарк ничего не покажет существенного, то может и нет смысла продолжать").
  Do not round a bad result into a good one, and do not claim validation succeeded if the numbers
  are ambiguous or worse for Rust. State plainly which of the three cases holds — Rust's
  per-point instruction count is (a) comparable to or better than C++ (validates ADR 0004 as
  written), (b) meaningfully worse (the design has a real, measured cost), or (c) the benchmark
  itself is inconclusive (e.g. both toggle setups failed, or the two binaries could not be built
  under identical conditions) — and stop there. Do not propose which of the ADR's own fallback
  options (revise the design vs. accept the cost) to take, and do not ask the user to choose
  between continuing the port or not: the user already stated the decision rule themselves
  ("если бенчмарк ничего не покажет существенного, то может и нет смысла продолжать"), so a
  plain, honest report of the result is the deliverable — the decision on how to act on it is
  theirs to make once they have it, not a question to route back to them.
