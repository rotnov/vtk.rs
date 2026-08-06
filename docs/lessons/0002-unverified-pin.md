---
id: '0002'
date: 2026-08-06
title: The pinned reference was not the pinned version
status: open
enforced_by:
---

# 0002 — The pinned reference was not the pinned version

**What happened.** Every document said the reference tree was pinned at `v9.6.2`.
`CMake/vtkVersion.cmake` read major 9, minor 7, `VTK_BUILD_VERSION 20260806` — a dated build
version, i.e. a development snapshot — and the repository carried no tags at all.

**Cause.** The pin was asserted in prose and never verified against the tree it described.

**What would have caught it.** Two commands: `grep` on `vtkVersion.cmake`, and
`git describe --tags`. Both now sit in `AGENTS.md` § Upstream version next to the claim, so the
assertion and its test travel together.

**Still open.** Nothing yet fails if the tree stops matching the documented pin. A CI check
asserting `git merge-base --is-ancestor <pin> HEAD` and the version triple would retire this.
