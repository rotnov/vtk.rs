---
id: '0001'
date: 2026-08-06
title: A document certified itself
status: enforced
enforced_by: 'Evidence quoted inline in ROADMAP.md; ledger-check for test claims'
---

# 0001 — A document certified itself

**What happened.** `ROADMAP.md` stated that its module order was "verified by reading the actual
files, not assumed". It was not. Three of Phase 1's dependency claims were wrong — `CommonMath`
also depends on `kissfft`, and `CommonTransforms` and `CommonMisc` both depend on `CommonMath` —
and Phases 2 and 3 each omitted prerequisite modules entirely.

**Cause.** The assurance was written alongside the work instead of produced by it. Nothing in the
document could be re-checked without redoing the whole analysis from scratch.

**What would have caught it.** Quoting each module's `DEPENDS` inline, so the claim carries its
own evidence.

**Generalisation.** A self-certification with no reproducible artifact is worth less than no
claim at all: it actively discourages the check it could not survive.
