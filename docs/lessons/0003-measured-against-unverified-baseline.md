---
id: '0003'
date: 2026-08-06
title: Measured against an unverified baseline, then blamed the measurements
status: open
enforced_by:
---

# 0003 — Measured against an unverified baseline, then blamed the measurements

**What happened.** A review reported that none of `ROADMAP.md`'s Snapshot counts reproduced.
After re-pinning to the real `v9.6.2`, four of five reproduced *exactly*. The counts had been
right all along; the tree was wrong.

**Cause.** Measurement was performed against a baseline that had not itself been checked, and the
discrepancy was attributed to the numbers rather than to the baseline. See [0002](0002-unverified-pin.md) —
the same root cause produced two distinct failures.

**What would have caught it.** Verifying the baseline before drawing any conclusion from things
measured against it.

**Generalisation.** When a whole set of independent figures disagrees in the same direction,
suspect the ruler, not the measurements.
