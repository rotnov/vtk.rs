# Status Dashboard v0 (Porting-Progress Panel) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship a GitHub Pages site that shows one live stat — the percentage of catalogued tests
with `status=ported` in `docs/test-mapping.csv` — rebuilt from repo state on every push to
`master`, per the approved design at
`docs/superpowers/specs/2026-08-07-status-dashboard-design.md` (issue
[#35](https://github.com/rotnov/vtk.rs/issues/35)).

**Architecture:** A `uv`-run Python generator (`.github/scripts/generate_dashboard.py`) reads
`docs/test-mapping.csv`, computes the stat with a pure function, and renders one self-contained
static HTML file. A new GitHub Actions workflow (`.github/workflows/pages.yml`) runs the generator
on every push to `master` and publishes its output via the official Pages Actions
(`configure-pages` / `upload-pages-artifact` / `deploy-pages`). Nothing is ever committed to the
repository — the site is fully regenerated from repo state on every run.

**Tech Stack:** Python 3.12+ run via `uv` (PEP 723 inline script metadata, no `pyproject.toml` or
lockfile), `pytest` for the generator's own unit tests, GitHub Actions with the official Pages
deployment actions.

## Global Constraints

- Design source of truth: `docs/superpowers/specs/2026-08-07-status-dashboard-design.md`. This
  plan implements that spec exactly; where this plan and the spec ever disagree, the spec governs
  and the disagreement should be flagged, not silently resolved either way.
- Writable paths (`AGENTS.md` § What is writable, verbatim): "Writable: `rust/`, `docs/`,
  `.claude/` ..., `.github/workflows/` and `.github/scripts/` (our CI and the scripts it runs ...),
  and the project meta-files at the repository root that are *not* part of upstream VTK —
  currently `AGENTS.md`, `ROADMAP.md`, `CLAUDE.md`." This plan touches only
  `.github/scripts/generate_dashboard.py`, `.github/scripts/tests/test_generate_dashboard.py`,
  `.github/scripts/.gitignore`, and `.github/workflows/pages.yml` — all inside the allowlist. No
  change to `paths_check.py`'s `ALLOWED_DIRS` is needed.
- The test-mapping ledger's exact CSV header and status enum (`AGENTS.md` § The test-mapping
  ledger, verbatim):
  ```csv
  original_path,original_test,original_sha,rust_path,rust_test,category,status,notes
  ```
  `status` values: `deferred` · `spec` · `ported` · `skipped`. Only `status == "ported"` counts
  toward the numerator. Today `docs/test-mapping.csv` is header-only (zero data rows) — this plan
  does not add rows to it.
- **Compute everything, store nothing.** No database, no committed JSON, no cache. Every render
  starts fresh from `docs/test-mapping.csv` as it exists in the triggering commit.
- **New Python tooling in this repo uses `uv`, not `pip`.** This is a standing project convention
  (distinct from the two older scripts, `paths_check.py`/`check_ascii.py`, which predate it and are
  not touched here). Both the generator and its test file carry their own PEP 723 inline metadata
  block; there is no `requirements-dev.txt` entry and no `pyproject.toml` for this feature.
- **Build output path:** `.github/scripts/_site/index.html`. Never a repo-root `_site/` — that
  would need an entry in the upstream-owned root `.gitignore`, which this project cannot touch. A
  nested `.github/scripts/.gitignore` (matching this repo's existing pattern of directory-scoped
  `.gitignore` files, e.g. none yet under `.github/scripts/` but the pattern is established
  elsewhere in the repo) keeps `_site/` untracked.
- **The disposable-branch positive-control smoke test convention does not apply here.** Per the
  spec's § Testing/verification: that convention targets PR gates; `pages.yml` triggers only on
  `push: branches: [master]`, so a commit on a scratch branch never invokes it. This plan's
  verification is the spec's three-part alternative instead: (1) the generator's own unit tests as
  the positive control for compute logic, (2) one real end-to-end check after this plan's PR
  merges (build+deploy go green, the live Pages URL shows the correct content), (3) a local
  dry-run failure check (point the generator at a missing `docs/test-mapping.csv`, confirm
  non-zero exit) run directly in a checkout, not through Actions.
- **One repository-setting change requires the owner's explicit confirmation before it runs**:
  flipping GitHub Pages' source to "GitHub Actions" (`gh api -X POST repos/rotnov/vtk.rs/pages -f
  build_type=workflow`, or the equivalent Settings UI toggle). This is a live repo-config change,
  not a file in the tree, and per the spec must happen before this plan's PR merges to `master`
  (the first `deploy-pages` run fails on this precondition otherwise). It is called out as its own
  controller-executed step below, never bundled into an implementer task.
- Out of scope (per the spec's § Out of scope): every panel besides porting-progress, per-PR
  preview deploys, any client-side JS or interactivity, moving the generator into `cargo xtask`,
  unifying the two older scripts onto `uv`, historical trend / charting.

---

### Task 1: Generator pure-logic layer (`compute_progress`, `render_html`)

**Files:**
- Create: `.github/scripts/generate_dashboard.py` (pure functions only — no I/O yet)
- Create: `.github/scripts/tests/test_generate_dashboard.py`

**Interfaces:**
- Consumes: nothing from earlier tasks (first task).
- Produces: `compute_progress(rows: list[dict]) -> dict` returning
  `{"total": int, "ported": int, "percent": float | None}`; `render_html(stats: dict, commit_sha:
  str, generated_at: str) -> str` returning a complete HTML document as a string. Task 2's `main()`
  calls both by these exact names and signatures.

- [ ] **Step 1: Write the failing tests for `compute_progress`**

Create `.github/scripts/tests/test_generate_dashboard.py`:

```python
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from generate_dashboard import compute_progress


def test_compute_progress_empty_rows():
    assert compute_progress([]) == {"total": 0, "ported": 0, "percent": None}


def test_compute_progress_all_deferred():
    rows = [{"status": "deferred"}, {"status": "deferred"}]
    result = compute_progress(rows)
    assert result == {"total": 2, "ported": 0, "percent": 0.0}


def test_compute_progress_mixed_statuses():
    rows = [
        {"status": "ported"},
        {"status": "ported"},
        {"status": "deferred"},
        {"status": "spec"},
        {"status": "skipped"},
    ]
    result = compute_progress(rows)
    assert result == {"total": 5, "ported": 2, "percent": 40.0}


if __name__ == "__main__":
    import pytest

    raise SystemExit(pytest.main([__file__, "-v"]))
```

The `if __name__ == "__main__"` block lets this file run standalone as `uv run
.github/scripts/tests/test_generate_dashboard.py` (Step 2 of this task explains why).

- [ ] **Step 2: Run the tests to verify they fail**

`.github/scripts/generate_dashboard.py` doesn't exist yet, so this must fail on import, not on an
assertion. Run:

```bash
cd .github/scripts && python3 -m pytest tests/test_generate_dashboard.py -v
```

Expected: `ModuleNotFoundError: No module named 'generate_dashboard'` (or similar import error) —
not an `AssertionError`. If you see an `AssertionError` here, something already exists at that
path; stop and check before continuing.

- [ ] **Step 3: Implement `compute_progress` and `render_html`**

Create `.github/scripts/generate_dashboard.py`:

```python
# /// script
# requires-python = ">=3.12"
# dependencies = []
# ///
"""Generate the status-dashboard v0 static page: one porting-progress stat.

Reads docs/test-mapping.csv and renders a single self-contained HTML file to
.github/scripts/_site/index.html. Computes everything from repo state at build time; stores
nothing. See docs/superpowers/specs/2026-08-07-status-dashboard-design.md.
"""


def compute_progress(rows):
    total = len(rows)
    ported = sum(1 for row in rows if row.get("status") == "ported")
    percent = (ported / total * 100) if total > 0 else None
    return {"total": total, "ported": ported, "percent": percent}


def render_html(stats, commit_sha, generated_at):
    total = stats["total"]
    ported = stats["ported"]
    percent = stats["percent"]
    if percent is None:
        headline = "0 catalogued tests yet"
    else:
        headline = f"{percent:.1f}% ported ({ported}/{total} catalogued tests)"
    return f"""<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>vtk2rust porting progress</title>
<style>
  body {{
    font-family: system-ui, sans-serif;
    background: #0b0d12;
    color: #e6e6e6;
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100vh;
    margin: 0;
  }}
  main {{ text-align: center; }}
  h1 {{ font-size: 2.5rem; margin: 0 0 0.5rem; font-weight: 600; }}
  footer {{ margin-top: 2rem; color: #888; font-size: 0.85rem; }}
</style>
</head>
<body>
<main>
<h1>{headline}</h1>
<footer>generated from commit {commit_sha} at {generated_at}</footer>
</main>
</body>
</html>
"""
```

Note the PEP 723 block (`# /// script ... # ///`) must be the very first thing in the file, before
the module docstring — that is where `uv` looks for it.

- [ ] **Step 4: Run the tests to verify they pass**

```bash
cd .github/scripts && python3 -m pytest tests/test_generate_dashboard.py -v
```

Expected: 3 passed.

- [ ] **Step 5: Commit**

```bash
git add .github/scripts/generate_dashboard.py .github/scripts/tests/test_generate_dashboard.py
git commit -m "feat: dashboard generator pure logic (compute_progress, render_html)"
```

---

### Task 2: Generator I/O layer (`main`), `.gitignore`, local verification

**Files:**
- Modify: `.github/scripts/generate_dashboard.py` (add `main()` and the `if __name__ ==
  "__main__"` entry point below the two pure functions from Task 1)
- Create: `.github/scripts/.gitignore`

**Interfaces:**
- Consumes: `compute_progress` and `render_html` from Task 1, same file.
- Produces: running `uv run .github/scripts/generate_dashboard.py` from the repo root writes
  `.github/scripts/_site/index.html`. Task 3's workflow calls this exact command from the repo
  root (not from inside `.github/scripts/`), so `main()` must resolve `docs/test-mapping.csv` and
  the output directory relative to the current working directory, matching how the two existing
  check scripts are invoked from the repo root in their own workflow.

- [ ] **Step 1: Add `.github/scripts/.gitignore`**

```
_site/
```

This keeps the build output untracked if a contributor runs the generator locally, without
touching the upstream-owned root `.gitignore`.

- [ ] **Step 2: Add `main()` to `generate_dashboard.py`**

Replace the full file with this (Task 1's `compute_progress`/`render_html` are unchanged, with
imports added above them and `main()` added below):

```python
# /// script
# requires-python = ">=3.12"
# dependencies = []
# ///
"""Generate the status-dashboard v0 static page: one porting-progress stat.

Reads docs/test-mapping.csv and renders a single self-contained HTML file to
.github/scripts/_site/index.html. Computes everything from repo state at build time; stores
nothing. See docs/superpowers/specs/2026-08-07-status-dashboard-design.md.
"""

import csv
import os
import subprocess
from datetime import datetime, timezone
from pathlib import Path


def compute_progress(rows):
    total = len(rows)
    ported = sum(1 for row in rows if row.get("status") == "ported")
    percent = (ported / total * 100) if total > 0 else None
    return {"total": total, "ported": ported, "percent": percent}


def render_html(stats, commit_sha, generated_at):
    total = stats["total"]
    ported = stats["ported"]
    percent = stats["percent"]
    if percent is None:
        headline = "0 catalogued tests yet"
    else:
        headline = f"{percent:.1f}% ported ({ported}/{total} catalogued tests)"
    return f"""<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>vtk2rust porting progress</title>
<style>
  body {{
    font-family: system-ui, sans-serif;
    background: #0b0d12;
    color: #e6e6e6;
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100vh;
    margin: 0;
  }}
  main {{ text-align: center; }}
  h1 {{ font-size: 2.5rem; margin: 0 0 0.5rem; font-weight: 600; }}
  footer {{ margin-top: 2rem; color: #888; font-size: 0.85rem; }}
</style>
</head>
<body>
<main>
<h1>{headline}</h1>
<footer>generated from commit {commit_sha} at {generated_at}</footer>
</main>
</body>
</html>
"""


def main():
    csv_path = Path("docs/test-mapping.csv")
    with csv_path.open(newline="", encoding="utf-8") as f:
        rows = list(csv.DictReader(f))

    stats = compute_progress(rows)

    commit_sha = os.environ.get("GITHUB_SHA")
    if not commit_sha:
        commit_sha = subprocess.check_output(
            ["git", "rev-parse", "HEAD"], text=True
        ).strip()

    generated_at = datetime.now(timezone.utc).strftime("%Y-%m-%d %H:%M UTC")

    html = render_html(stats, commit_sha, generated_at)

    out_dir = Path(".github/scripts/_site")
    out_dir.mkdir(parents=True, exist_ok=True)
    out_path = out_dir / "index.html"
    out_path.write_text(html, encoding="utf-8")

    print(f"generate_dashboard: wrote {out_path} ({stats})")


if __name__ == "__main__":
    main()
```

If `docs/test-mapping.csv` is missing or unparseable, the `csv_path.open(...)` or
`csv.DictReader` call raises, `main()` does not catch it, and the process exits non-zero. This is
intentional (see this plan's Global Constraints and the spec's § Error handling) — do not add a
try/except here.

- [ ] **Step 3: Run it locally against the real (header-only) ledger**

From the repo root:

```bash
uv run .github/scripts/generate_dashboard.py
cat .github/scripts/_site/index.html
```

Expected: exits 0, prints a `generate_dashboard: wrote ...` line with
`{'total': 0, 'ported': 0, 'percent': None}`, and the HTML file's `<h1>` contains exactly `0
catalogued tests yet`.

- [ ] **Step 4: Run the local dry-run failure check**

This is this plan's substitute for the disposable-branch smoke test (see Global Constraints) —
run once now, by hand, not as an automated test:

```bash
mv docs/test-mapping.csv docs/test-mapping.csv.bak
uv run .github/scripts/generate_dashboard.py; echo "exit code: $?"
mv docs/test-mapping.csv.bak docs/test-mapping.csv
```

Expected: `exit code:` is non-zero (a `FileNotFoundError` traceback prints first — that's
correct, it's the uncaught exception this design calls for). Confirm `docs/test-mapping.csv` is
restored before continuing (the second `mv` does this).

- [ ] **Step 5: Run the generator's own tests via `uv`**

```bash
uv run .github/scripts/tests/test_generate_dashboard.py
```

Expected: 3 passed (same tests as Task 1, now confirmed runnable the way CI will run them in
Task 3 — via `uv run` against the file's own PEP 723 metadata, not `pip`/`python3 -m pytest`).
`test_generate_dashboard.py` needs its own PEP 723 header for this to resolve `pytest` — add it as
the very first line of the file, before the existing `import sys`:

```python
# /// script
# requires-python = ">=3.12"
# dependencies = ["pytest"]
# ///
```

- [ ] **Step 6: Clean up the local build artifact and commit**

```bash
rm -rf .github/scripts/_site
git add .github/scripts/generate_dashboard.py .github/scripts/tests/test_generate_dashboard.py .github/scripts/.gitignore
git commit -m "feat: dashboard generator I/O layer, gitignore build output"
```

---

### Task 3: `pages.yml` GitHub Actions workflow

**Files:**
- Create: `.github/workflows/pages.yml`

**Interfaces:**
- Consumes: `.github/scripts/generate_dashboard.py` and
  `.github/scripts/tests/test_generate_dashboard.py` from Tasks 1-2, run via `uv` exactly as
  verified locally in Task 2 Steps 3 and 5.
- Produces: nothing further consumes this — it's the last file this plan adds.

- [ ] **Step 1: Add `.github/workflows/pages.yml`**

```yaml
name: pages

on:
  push:
    branches: [master]

concurrency:
  group: pages
  cancel-in-progress: false

permissions:
  contents: read

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - uses: astral-sh/setup-uv@v5

      - name: Run generator's own tests
        run: uv run .github/scripts/tests/test_generate_dashboard.py

      - name: Run generator
        run: uv run .github/scripts/generate_dashboard.py

      - uses: actions/configure-pages@v5

      - uses: actions/upload-pages-artifact@v3
        with:
          path: .github/scripts/_site

  deploy:
    needs: build
    runs-on: ubuntu-latest
    permissions:
      pages: write
      id-token: write
    environment:
      name: github-pages
      url: ${{ steps.deployment.outputs.page_url }}
    steps:
      - id: deployment
        uses: actions/deploy-pages@v4
```

Before committing, verify `astral-sh/setup-uv`, `actions/configure-pages`,
`actions/upload-pages-artifact`, and `actions/deploy-pages` are still pinned to their current
recommended major version tags in GitHub's own documented Pages-via-Actions flow and in
`astral-sh/setup-uv`'s README — update the tags above if a newer major exists at implementation
time. This mirrors the design spec's own explicit hedge on these version pins (§ Workflow): the
spec is not the place that stays current with upstream Action releases, and neither is this plan.

- [ ] **Step 2: Commit**

```bash
git add .github/workflows/pages.yml
git commit -m "feat: add pages.yml workflow to build and deploy the status dashboard"
```

---

## Before merging this plan's PR: flip the Pages source setting (controller-executed, owner confirmation required)

Not a task — this does not touch a file in the tree, and per the spec it needs the owner's
explicit go-ahead before it runs, the same way any live repository-configuration change does.

1. Ask the owner to confirm before proceeding: "This PR adds `pages.yml`, which needs GitHub Pages'
   source set to 'GitHub Actions' before it merges (otherwise the first `deploy-pages` run fails on
   that precondition). OK to run `gh api -X POST repos/rotnov/vtk.rs/pages -f
   build_type=workflow` now?"
2. Only after explicit confirmation, run:
   ```bash
   gh api -X POST repos/rotnov/vtk.rs/pages -f build_type=workflow
   ```
3. Confirm the change took effect:
   ```bash
   gh api repos/rotnov/vtk.rs/pages --jq '.build_type'
   ```
   Expected: `workflow`.

## After merge: end-to-end verification (controller-executed, not a task)

Per this plan's Global Constraints, the disposable-branch smoke-test convention doesn't apply to a
push-triggered, non-gating workflow. Instead:

1. Merge this plan's PR to `master` (only after the Pages-source step above is confirmed done).
2. Watch the `pages` workflow's run on `master`:
   ```bash
   gh run list --repo rotnov/vtk.rs --workflow pages.yml --branch master --limit 1
   ```
   Confirm both `build` and `deploy` jobs show `success`.
3. Fetch the live Pages URL from the `deploy` job's output (or `gh api repos/rotnov/vtk.rs/pages
   --jq '.html_url'`) and confirm the page renders `0 catalogued tests yet` (today's true state,
   `docs/test-mapping.csv` being header-only) — not a 404, a blank page, or stale cached content.
4. Record the workflow run URL and the confirmed live Pages URL in this plan document, mirroring
   how `docs/superpowers/plans/2026-08-07-wasm-check-common.md` records its smoke-test run URLs,
   since this is the only record that this workflow was ever proven to work end-to-end.
