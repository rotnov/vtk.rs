---
id: '0006'
date: 2026-08-06
title: A new rule silently weakened an existing one
status: open
enforced_by:
---

# 0006 — A new rule silently weakened an existing one

**What happened.** Permitting tests we write ourselves made the coverage gate stop enforcing the
porting order. `AGENTS.md` § Coverage still claimed "uncovered code means you ported ahead of the
tests", but an own test can cover code no VTK test exercises — so an agent could reach green CI
having ported nothing at all.

**Cause.** A rule was added without re-reading the rules it interacts with.

**What would have caught it.** Asking, before adding a rule, which existing rule it weakens.

**Outcome.** Split into two gates: coverage satisfied by any test, parity satisfied only by
ported tests. The parity gate is being mechanised as a fourth `ledger-check` assertion; until
that lands, this stays open.
