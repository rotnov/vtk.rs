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
