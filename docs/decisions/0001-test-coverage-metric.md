# 0001 — Which coverage metric the 100% gate applies to

Status: accepted
Date: 2026-08-06

## Context

The project requires hard 100% test coverage, enforced in CI on the default branch. "100%
coverage" is ambiguous on its own: `cargo llvm-cov`, the standard source-based coverage tool for
Rust, reports **lines**, **functions**, and **regions** as three separate percentages, and they
are not equally reachable.

Region coverage counts LLVM coverage mapping regions, which include code the crate never wrote
and no test can drive:

- `#[derive(...)]` expansions (`Debug`, `Clone`, `PartialEq`) generate arms per field and per
  variant; exhaustively driving all of them says nothing about the port's correctness.
- Panic paths — `unwrap`, `expect`, `assert!`, `unreachable!()`, slice bounds checks — are
  regions that exist precisely so they are never taken.
- Generic code monomorphizes per instantiation. Regions in instantiations that a given feature
  set never produces are counted but unreachable in that build.

An unqualified "100%" in `AGENTS.md` would be read cold by the next agent, who would run the
default `llvm-cov` summary, see regions below 100%, and resolve it the wrong way: contorted
tests that assert nothing, or silent `#[coverage(off)]` / `--ignore-filename-regex` exclusions
that hollow the gate out while it still reports green.

## Decision

The 100% gate applies to **lines and functions**. Regions are measured and reported, not gated.

```sh
cargo llvm-cov --workspace --all-features \
  --fail-under-lines 100 --fail-under-functions 100
```

Coverage exclusions of any kind (`#[coverage(off)]`, `--ignore-filename-regex`, and equivalents)
require their own ADR naming the file and the reason. There are none today.

## Consequences

- The gate is achievable without writing tests that exist only to move a number, so it stays
  credible and stays enforced.
- Line + function coverage at 100% means every ported function is executed by at least one
  ported test. Combined with the port-tests-first workflow, that makes the gate the enforcement
  mechanism for the porting order: code no ported test exercises must not be written yet. See
  `AGENTS.md` § Change workflow.
- Regions being ungated leaves a real gap: a fully line-covered `match` can still have untaken
  arms. Mitigated by porting VTK's own tests rather than writing minimal tests to satisfy the
  tool — VTK's suites exercise branches, not just entry points.
- If region coverage is later wanted as a gate, it needs a threshold below 100% and a superseding
  ADR. Do not silently raise or lower these numbers.

## How this survives the TDD workflow

*Amended 2026-08-06. Clarification, not a reversal — the metric above is unchanged.*

A hard gate and a test-first workflow are usually assumed to fight. They don't here, and the
reason is worth stating: 100% coverage is normally unreachable because code gets written ahead of
its tests and accumulates branches nothing calls. Under port-tests-first that code never exists,
so the gate is a ratchet holding a property the workflow already produces, not a target to chase.

The red phase never reaches CI. `cargo test` fails locally while the implementation is missing;
what gets pushed is green, and the gate is a required check on merge, so a draft PR may sit red
for as long as it takes.

Three things do break the gate for reasons unrelated to real coverage. All three are handled by
convention, not by exclusions:

1. **`#[ignore]`d spec tests.** `AGENTS.md` § Testing strategy has category-2 tests ported as
   specs before the module exists. An ignored test body never executes, so its lines count as
   uncovered. **Deferred and `#[ignore]`d specs live in `tests/`**, which `cargo-llvm-cov`
   excludes from the report by default (alongside `examples/` and `benches/`). Unit tests that
   run live in `#[cfg(test)]` modules under `src/`. A spec moving from `tests/` into the covered
   set, un-ignored, is exactly the event that marks its module implemented.

2. **Boilerplate no test touches** — `#[derive(Debug)]`, `impl Display`, error-enum variants
   never constructed. These are uncovered lines and the gate is right to fail on them: an error
   variant no test constructs is an error variant no test proves you can produce. Exercise the
   derive, construct the variant. Do not reach for an exclusion; that is what hollows a coverage
   gate out until it reports green over nothing.

3. **An empty crate scores 100%.** The gate cannot tell "everything is covered" from "there is
   nothing here", so on its own it would bless a skeleton. `docs/test-mapping.csv` is the second
   signal: coverage says the code that exists is exercised, the ledger says how much of VTK's
   suite that code answers for. Neither is meaningful alone. Progress claims cite both.

## Alternatives rejected

- **Gate regions at 100%** — not reachable, for the reasons above.
- **Gate regions at a soft threshold now** (e.g. 90%) — an arbitrary number with no basis before
  a single crate exists. Revisit once Phase 1 has real code.
- **Gate lines only** — a module can hold a never-called public function whose body is covered
  through another path; the function gate closes that.
