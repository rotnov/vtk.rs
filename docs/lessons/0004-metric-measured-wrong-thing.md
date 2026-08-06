---
id: '0004'
date: 2026-08-06
title: A metric measured the wrong thing entirely
status: open
enforced_by:
---

# 0004 — A metric measured the wrong thing entirely

**What happened.** Pure-logic tests were counted by grepping for `NO_DATA NO_VALID NO_OUTPUT`,
and the hit count was reported as a number of directories. It is a number of *lines*, spread
across a different number of directories, and each line covers a whole block — one hit in
`Common/Core/Testing/Cxx` accounts for ~95 tests. A second, per-test comma syntax
(`TestFoo.cxx,NO_DATA,NO_VALID`) was invisible to the grep entirely. Phase 1 was understated by
about an order of magnitude.

**Cause.** A grep was designed against one observed syntax, without reading the API being grepped
for.

**What would have caught it.** Reading how `vtk_add_test_cxx` actually accepts its arguments
before counting them.

**Generalisation.** Before measuring a codebase by pattern, read the thing that generates the
pattern.
