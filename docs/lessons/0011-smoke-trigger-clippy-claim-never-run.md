---
id: '0011'
date: 2026-08-07
title: A plan's smoke-test trigger asserted clippy behavior nobody had run
status: promoted
enforced_by: docs/superpowers/plans/2026-08-07-rust-workspace-ci.md (amendment)
---

# 0011 — A plan's smoke-test trigger asserted clippy behavior nobody had run

**What happened.** The implementation plan for dependency-order Step 2 specified
`#[test] fn smoke_fails() { assert!(false); }` as the positive-control trigger to prove the new
`cargo-test` CI job fires red on a real violation, with the explicit claim (recorded during the
plan's own advisor review) that "an `assert!(false)` test doesn't fail clippy." Running the actual
positive-control sequence against live CI showed both `cargo-test` **and** `cargo-clippy` going
red on that commit: `assert!(false)` trips `clippy::assertions_on_constants`, which fires on any
`assert!`/`debug_assert!` given a literal boolean constant and is denied under this project's
`-D warnings` gate. The claim was false, and it survived a full plan self-review and an advisor
pass unverified.

**Cause.** Same root cause as `docs/lessons/0010-adr-tool-claim-never-run.md`, one document over:
a claim about a specific tool's behavior in a specific state ("this Rust snippet does/doesn't trip
clippy") was accepted on reasoning, not on a command actually run. Lesson 0010 scoped its fix to
ADRs and decision documents — "any ADR clause making an empirical claim about tool behavior." This
instance shows the same discipline was still missing one level up: plan documents embed shell
commands and code snippets whose entire purpose is to produce a specific, checkable tool outcome
(a lint firing, a test failing, a formatter rejecting), and those claims are exactly as unverified
as an ADR's until someone runs them against the real toolchain.

**What would have caught it.** Running `cargo clippy --workspace --all-targets --all-features
-- -D warnings` against the exact proposed skeleton with the exact proposed smoke-test snippet
inserted, before writing the trigger into the plan — the same "No Placeholders" verification bar,
extended past ADRs to any plan step whose entire job is to make a specific tool produce a specific
red/green outcome. A trigger snippet in a smoke-test plan section is an empirical claim wearing
the shape of example code; it does not get to skip verification because it looks like a one-liner.

**Outcome.** Caught mid-execution, not after merge: the live smoke-test run on the disposable
`smoke/rust-checks-red` branch showed the unexpected second failure before anything was merged, so
no red state reached `master`. The trigger was corrected in place — from `assert!(false)` to
`assert_eq!(1, 2)`, a runtime comparison rather than a constant-literal assertion — verified locally
against the real workspace (`cargo clippy` exit 0, `cargo fmt --all --check` exit 0, `cargo test`
fails as intended) before being pushed to the smoke branch and before the plan document was
amended to match. `docs/superpowers/plans/2026-08-07-rust-workspace-ci.md`'s smoke-test section now
states the verified trigger and records why `assert!(false)` was rejected, so a future reader does
not need to rediscover this. Promoted directly (no `open` waiting period) because the fix is the
plan text itself — a corrected rule now living in a doc under `docs/`, the same reasoning `0010`
used. Not `enforced`: no CI check makes a future plan author re-commit this exact mistake
impossible, only less likely.
