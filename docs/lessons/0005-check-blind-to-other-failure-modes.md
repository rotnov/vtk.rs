---
id: '0005'
date: 2026-08-06
title: A check designed from one failure mode was blind to the others
status: enforced
enforced_by: 'cargo xtask ledger-check (exists / complete / fresh)'
---

# 0005 — A check designed from one failure mode was blind to the others

**What happened.** `ledger-verify` asserted only that every `original_path` still exists. That
catches upstream deletions and renames. Tests *added* upstream were invisible — no row names
them, so the check iterates straight past. Tests *rewritten* upstream were invisible too: path
and row both still look correct while the port silently diverges at green CI.

**Cause.** The check was designed from the first failure mode that came to mind, rather than from
an enumeration of what can happen to an upstream test.

**What would have caught it.** Enumerating the state space — added, removed, changed — before
designing the guard.

**Outcome.** Became three assertions: *exists*, *complete*, *fresh*, plus an `original_sha`
column. See `docs/decisions/0003-upstream-sync-strategy.md`.
