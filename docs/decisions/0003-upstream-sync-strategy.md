# 0003 — How the pinned upstream version is advanced

Status: accepted
Date: 2026-08-06

## Context

The reference tree is pinned at `v9.6.2`. `AGENTS.md` forbids moving it silently but says nothing
about moving it deliberately, so the first bump would be improvised — and the parts that rot on a
version bump are precisely the ones nobody thinks to check.

Three of them, concretely:

- `docs/test-mapping.csv` rows whose `original_path` names a file upstream has moved, renamed, or
  deleted. Nothing detects this; the ledger keeps reporting parity against tests that no longer
  exist.
- Ported tests whose upstream original has been rewritten. Ours still passes, so CI is green,
  while the behaviour it claims parity with has changed underneath.
- Source changes inside modules already ported. A bug fixed upstream is a bug still present here,
  and no signal distinguishes that from an unrelated change elsewhere in the tree.

This project has already been bitten once by the reference tree not being what the documents said
it was (see `AGENTS.md` § Upstream version). The lesson taken is that pin-related claims must be
mechanically checkable, not asserted.

## Decision

### Mechanic: merge, with rebase as the stated fallback

Advance by merging the new tag into `master`. Our commits touch only paths that do not exist
upstream, so the merge is conflict-free by construction; any conflict is a signal that something
was written outside the writable paths in `AGENTS.md` § What is writable, and is a bug to fix
rather than a merge to resolve.

This works only while the new tag is a descendant of the current pin. Verified for the case in
hand — `v9.6.2` is a linear ancestor of `v9.7.0.rc4`, 2595 commits ahead and 0 behind — but VTK
tags releases off release branches, so it is not guaranteed in general. **Check it every time:**

```sh
git fetch --no-tags upstream tag vX.Y.Z
git merge-base --is-ancestor <current-pin> vX.Y.Z && echo linear || echo diverged
```

If diverged, rebase our commits onto the new tag instead
(`git rebase --onto vX.Y.Z <old-base>`) and force-push. That rewrites public history and is a
deliberate, announced act, not a routine one.

### The bump is driven by a bucketed diff, cross-referenced with the ledger

`git diff --name-status <old-tag> <new-tag>`, grouped by module, answers four different questions
that imply four different kinds of work:

| what changed | what it means |
|---|---|
| test added upstream | new ledger row, `status=deferred`, triaged into a phase |
| test removed or renamed | our ported test is orphaned — keep it as a regression test or drop it, `notes` must say which and why |
| test content changed | our port may have silently diverged; re-port it and re-check the ledger row |
| source changed in an already-ported module | behaviour may have changed; re-read the diff against our implementation |

Only the ledger can say which of these touch us, which is why it has to be trustworthy before a
bump, not after.

### Ledger integrity is a CI check, not a discipline — and it takes three assertions

Add `cargo xtask ledger-check` to the required checks. One check walking one direction is not
enough; each of the three failures below is invisible to the other two.

- **exists** — every `original_path` in `docs/test-mapping.csv` is present in the reference tree.
  Catches tests upstream removed or renamed.
- **complete** — for every module with at least one ledger row, every test registered in that
  module's `Testing/*/CMakeLists.txt` has a row. Catches tests **added** upstream. Nothing else
  can: a new test is named by no row, so *exists* iterates right past it. Scoped to started
  modules deliberately — unscoped it fails on all 2388 untouched tests on day one, and a check
  that is red by default gets switched off rather than fixed.
- **fresh** — every row's `original_sha` still matches the blob SHA of its `original_path`.
  Catches upstream **rewriting** a test we already ported.

*fresh* is the one that justifies a schema change, and it is the most valuable of the three. When
upstream rewrites a ported test, the path exists and the row looks correct; our port keeps passing
the assertions it was written against, CI stays green, and the behaviour we claim parity with has
moved underneath. A removed test is noisy. This is silent, and silence is precisely what the
ledger exists to prevent.

It requires an `original_sha` column: the git blob SHA of the original file at port time. Per
file, not per test function, so rows sharing a file share a SHA and a change flags all of them.
Coarse on purpose — it costs one `git rev-parse`, needs no C++ parsing, and its failure mode is
re-reading a test that didn't really change, which is far cheaper than missing one that did.

Answering *fresh* means re-reading the upstream test and re-porting what changed, then updating
the SHA in the same commit. Updating the SHA alone deletes the alarm instead of answering it, and
is the one move that makes the whole ledger worthless.

### Versioning: our own semver, upstream recorded separately

The port is incomplete and will be for a long time, so our crates cannot claim VTK's version
number — `9.6.2` would assert a completeness we do not have. Instead:

- crates carry their own semver, `0.x` while any Phase 1–3 module is unported;
- the upstream tag ported against is recorded in one place, as a constant in `vtk-common-core`
  and in `ROADMAP.md` § Snapshot;
- our git tags carry both: `v0.1.0+vtk9.6.2`. Build metadata after `+` is semver-legal and
  ignored in version comparison, so it documents the pairing without affecting resolution.

### Cadence: deliberate, never mid-phase

Do not track upstream continuously. Bump when there is a reason — a needed fix, a module about to
be ported that changed substantially, or a release we want to claim parity with. Never bump in the
middle of a phase: it changes the target while modules are being ported against it.

Every bump gets its own ADR recording the old and new tags, the bucketed diff summary, and what it
cost.

## Consequences

- The bump becomes a reviewable, repeatable operation with a written output, instead of a large
  opaque commit.
- `ledger-check` will fail on the *first* bump, loudly, on all three assertions at once — moved
  tests, added tests, rewritten tests. That is the intended behaviour and the point of the check:
  the work is surfaced rather than skipped.
- Merging keeps history intact, so PR references and commit SHAs stay valid across bumps — unlike
  the one-off rebase that fixed the original mis-pin.
- The `+vtk9.6.2` tag convention means anyone can tell which upstream a release was built against
  without reading the source.
- Recording upstream in two places (a constant and `ROADMAP.md`) is duplication and can drift.
  Accepted deliberately: both are needed, one by code and one by readers. `ledger-check` is the
  backstop that catches the version being wrong in practice.

## Alternatives rejected

- **Track upstream `master` continuously.** Makes "what are we porting against" unanswerable at
  any given moment, which is the exact failure this repo already suffered.
- **Rebase on every bump.** Cleaner history, but rewrites public history routinely and invalidates
  every commit SHA referenced from issues and PRs. Reserved for the diverged case.
- **Version our crates as VTK's version.** Claims a parity we do not have, and leaves no room to
  release fixes to the port itself between upstream releases.
- **Vendor upstream as a git submodule instead of a merged tree.** Would make bumps trivial, but
  breaks the property this repo is built on: reading VTK sources and our port side by side in one
  checkout, with `git diff` between them meaningful.
