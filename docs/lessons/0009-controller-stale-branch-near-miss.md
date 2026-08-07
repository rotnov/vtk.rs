---
id: '0009'
date: 2026-08-07
title: The controller itself worked from a local branch two merged commits behind origin/master
status: open
enforced_by:
---

# 0009 — The controller itself worked from a local branch two merged commits behind origin/master

**What happened.** Resuming the standing ralph-loop directive in a fresh session, the local
worktree branch `19-autonomy-spec` was silently two commits behind `origin/master` — missing
`964a723055` (PR #22) and `8d7745ed90` (PR #24), both already merged by an earlier session. This
surfaced only as a confusing symptom: `docs/superpowers/plans/2026-08-07-paths-and-language-checks.md`
appeared not to exist, which briefly looked like a data-loss incident. Root cause was mundane —
the branch had simply never been fetched and fast-forwarded since those merges — and it was fixed
with `git merge --ff-only origin/master` after confirming zero local-only commits.

**Cause.** Nothing re-syncs a controller's own checked-out branch with `origin/master` between
sessions or after a merge it just performed elsewhere. Every prior lesson in this ledger
(0007, 0008) is about a *dispatched subagent* losing sync with the controller's intended worktree
state. This is the same failure one level up: the controller trusted its own branch tip without
verifying it against the remote first.

**What would have caught it.** Treating `git fetch origin --prune && git log --oneline
HEAD..origin/master` as a mandatory first action at the start of any resumed session or loop
iteration, before reading or editing any file — not just after a symptom (a missing file, a
confusing diff) prompts investigating it. Had a stale-branch edit happened to touch a file that
also changed in `964a723055` or `8d7745ed90`, a plain `git merge --ff-only` would have refused,
but a less careful merge or an `AGENTS.md` edit made from the stale base could have silently
reverted or conflicted with already-merged, CI-approved work.

**Outcome.** Fast-forwarded cleanly with no lost work, since `--ff-only` refuses on any divergence
and none existed. No check yet enforces the fetch-before-work step; it depends on the controller
remembering to run it. This lesson has not been promoted to a rule or a check.
