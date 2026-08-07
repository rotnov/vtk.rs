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

3. **An empty crate scores 100% — corrected 2026-08-07, see amendment below.** This is true only
   once at least one crate in the workspace has an executing test; an entire workspace with zero
   executing tests anywhere does not score 100%, it hard-errors. See the amendment for the
   verified behavior and what it means for wiring the coverage job.

## Alternatives rejected

- **Gate regions at 100%** — not reachable, for the reasons above.
- **Gate regions at a soft threshold now** (e.g. 90%) — an arbitrary number with no basis before
  a single crate exists. Revisit once Phase 1 has real code.
- **Gate lines only** — a module can hold a never-called public function whose body is covered
  through another path; the function gate closes that.

## Amendment: item 3 was never run

*Amended 2026-08-07. Corrects a factual claim; the metric decided above is unchanged.*

Item 3 above ("an empty crate scores 100%") was reasoning about how `cargo llvm-cov` ought to
behave, not a command actually run. Verified by dry-running the exact state
`docs/superpowers/specs/2026-08-06-autonomous-operation-design.md`'s dependency-order Step 2
produces — a 7-crate workspace, every `src/lib.rs` a doc comment only, zero `#[test]`s anywhere:

```
$ cargo llvm-cov --workspace --all-features --fail-under-lines 100 --fail-under-functions 100
error: failed to load coverage: '...': no coverage data found
error: failed to generate report: process didn't exit successfully: `.../llvm-cov report ...` (exit status: 1)
```

Adding one executing test anywhere in the workspace (a single `#[test]` in one crate) made the
identical command pass at 100%/100%/100%. The claim holds once *any* crate has an executing test —
crates with no code simply don't appear as rows in the per-file table — but not when *no* crate
does. Zero executed tests means zero `.profraw` instrumentation data exists to merge or report on;
the tool errors before any `--fail-under-*` threshold is evaluated. No flag changes this (checked
`--fail-under-*`, `--fail-uncovered-*`, `--no-report`, `--failure-mode`).

This has a second consequence beyond the bootstrap skeleton: item 1 above says deferred
`#[ignore]`d specs live under `tests/`, which `cargo-llvm-cov` excludes from the report by default.
A module whose first ported commits are only ignored specs — no `src/` unit test yet — hits the
same "no coverage data found" wall, not just a brand-new skeleton crate. This is a recurring state
of the port-tests-first workflow, not a one-time quirk.

**Corrected rule:** the coverage job (`cargo llvm-cov --workspace --all-features
--fail-under-lines 100 --fail-under-functions 100`) is not wired into CI until the same PR that
adds the first crate with an actually-executing test — a `src/` unit test, or a `tests/` spec once
un-ignored. Dependency-order Step 2 ships the `rust/` workspace skeleton with `cargo test`,
`cargo clippy -D warnings`, and `cargo fmt --check` as three green CI jobs; the coverage job is a
separate, not-yet-done item, wired in Phase 1's first module PR instead. This is not an exclusion
under this ADR's "no exclusions without an ADR" rule — no code is ever covered and later exempted;
the job does not exist yet, the same way `cargo xtask ledger-check` does not exist before
dependency-order Step 3 builds it. See `docs/lessons/0010-adr-tool-claim-never-run.md`.
