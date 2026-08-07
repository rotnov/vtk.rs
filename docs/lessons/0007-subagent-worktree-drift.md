---
id: '0007'
date: 2026-08-07
title: A dispatched subagent worked in the wrong git worktree
status: open
enforced_by:
---

# 0007 — A dispatched subagent worked in the wrong git worktree

**What happened.** During subagent-driven execution of the CI plan
(`docs/superpowers/plans/2026-08-07-paths-and-language-checks.md`), the Task 4
implementer was dispatched with a prose `Working directory: <path>` line at
the top of its prompt, same as Tasks 1-3. Unlike those three, it read files,
ran tests, and committed entirely inside a different, unrelated worktree
(`/Users/denis/projects/vtk2rust`, checked out on a stale local `master` with
no common ancestor with `origin/master`) instead of the intended one
(`.../worktrees/project-overview-ee671d` on branch
`21-paths-and-language-checks`). Discovered only when `review-package`
reported "0 commits" for the expected range. The commit's content was correct
and recoverable via `git cherry-pick`, but the mistake could as easily have
landed on a worktree with uncommitted user work.

**Cause.** The `Agent` tool has no structural working-directory parameter —
only a prose instruction inside the dispatch prompt. Three prior subagents
happened to honor it; nothing enforced that the fourth would. A dispatch
prompt is data the subagent reads and can misapply, not a binding contract.

**What would have caught it.** Making the first action of every dispatched
subagent a self-check, not an instruction to trust: `cd <path> && pwd && git
branch --show-current`, with an explicit "stop and report if the branch does
not match" before touching any file. A controller-side check — diffing
`git rev-parse HEAD` immediately after each dispatch against the value
recorded before it — would also have caught this within one task instead of
one task later.

**Outcome.** Recovered content via cherry-pick without touching the other
worktree's history; reset its accidental commit off the stray branch by hand
afterward. All subsequent dispatch prompts in this plan's execution (Task 5
onward) open with the self-check above as their literal first instruction.
Still open: no controller-side post-dispatch HEAD check exists yet, and nothing
mechanically prevents a future dispatch from omitting the self-check — this
lesson has not yet been promoted to a rule or a check.
