---
title: Lessons index
---

# Lessons

Incidents and what they taught, per `AGENTS.md` § Rule 1. One file per lesson, numbered, written
in the same commit as the fix.

Each file carries front matter so the state of the Rule 1 loop is machine-readable:

```yaml
id: '0006'
date: 2026-08-06
title: A new rule silently weakened an existing one
status: open | promoted | enforced
enforced_by: the check that retired it, when status is enforced
```

`status` is the escalation from Rule 1:

- **open** — recorded, not yet a rule. Nothing prevents a recurrence.
- **promoted** — it happened again, so it is now a rule in `AGENTS.md` or a doc under `docs/`.
- **enforced** — a CI check makes it impossible. `enforced_by` names the check.

The ratio of *enforced* to *open* is the honest measure of whether this project is actually
learning or merely journalling. Prose that never becomes a check is a lesson that will be learned
again.
