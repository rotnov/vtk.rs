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

## Alternatives rejected

- **Gate regions at 100%** — not reachable, for the reasons above.
- **Gate regions at a soft threshold now** (e.g. 90%) — an arbitrary number with no basis before
  a single crate exists. Revisit once Phase 1 has real code.
- **Gate lines only** — a module can hold a never-called public function whose body is covered
  through another path; the function gate closes that.
