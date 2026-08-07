---
id: '0010'
date: 2026-08-07
title: An accepted ADR asserted tool behavior nobody had run
status: promoted
enforced_by: docs/decisions/0001-test-coverage-metric.md (amendment)
---

# 0010 — An accepted ADR asserted tool behavior nobody had run

**What happened.** While planning dependency-order Step 2 (the `rust/`
workspace skeleton plus `cargo test`/`clippy`/`fmt`/coverage in CI), a
dry-run build of the exact proposed skeleton — 7 crates, every `src/lib.rs`
containing only a doc comment, zero `#[test]`s anywhere in the workspace —
was used to validate the plan's file contents before writing them down, per
`superpowers:writing-plans`' "No Placeholders" rule. Running the coverage
gate command from `docs/decisions/0001-test-coverage-metric.md` against that
dry-run —
`cargo llvm-cov --workspace --all-features --fail-under-lines 100 --fail-under-functions 100` —
did not report "100% of nothing." It hard-errored:
`error: failed to load coverage: '...': no coverage data found`, because
zero executed tests anywhere in the workspace means zero `.profraw`
instrumentation data exists to merge or report on. ADR 0001's own text
claims the opposite: "An empty crate scores 100%." That claim is false for
the specific, real state Step 2's first commit lands in — an entire
workspace with no executing test yet, not just one empty crate inside an
otherwise-tested workspace.

**Cause.** ADR 0001 was written and marked `accepted` based on reasoning
about how `cargo llvm-cov` *should* behave, not a command actually run
against the state it describes. The claim is subtly wrong: it holds once
*any* crate in the workspace has at least one executing test (crates with no
code simply don't appear as rows in the per-file table), but breaks when
*no* crate anywhere has one — precisely the bootstrap moment the ADR was
written to cover, and precisely the moment Step 2 needed it to hold. The
same false premise has a second, non-bootstrap consequence the ADR also
missed: item 1 of the same document says deferred `#[ignore]`d specs live
under `tests/`, which `cargo-llvm-cov` excludes from the report by default —
so a module whose first ported commits are only ignored specs (no `src/`
unit test yet) hits the identical "no coverage data found" wall, not just a
brand-new skeleton crate.

**What would have caught it.** Treating any ADR clause that makes an
empirical claim about tool behavior ("X scores Y", "the tool reports Z") as
unverified until the exact command has actually been run against the exact
state the clause describes, with the output captured — the same bar
`writing-plans`' "No Placeholders" rule already applies to plan file
contents, extended to the decision documents plans are built on. A decision
doc's `accepted` status should not be trusted to mean "the mechanism was
verified"; it means "the reasoning was accepted," which is not the same
claim.

**Outcome.** `docs/decisions/0001-test-coverage-metric.md` was amended in
place (not reverted) with the reproduction — command, exact failure, and the
same command passing at 100%/100%/100% once one executing test exists
anywhere in the workspace — and a corrected rule for the coverage CI job:
the job is not wired into CI until the same PR that adds the first crate
with an actually-executing test (either a `src/` unit test or an un-ignored
`tests/` spec). Dependency-order Step 2 ships the workspace skeleton with
`cargo test`/`clippy -D warnings`/`fmt --check` as three green jobs; the
coverage job is a `[ ]` item on `ROADMAP.md` Phase 0, wired in Phase 1's
first module PR instead of Step 2's. This is not a bypass or an exclusion
under ADR 0001's own "no exclusions without an ADR" rule — no code was ever
covered and later exempted; the job simply does not exist yet, the same way
`cargo xtask ledger-check` does not exist until Step 3 builds it. Promoted
directly to `enforced` (no `open`/`promoted` waiting period) because the fix
is the ADR text itself, not a separate check to build later.
