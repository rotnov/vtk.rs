# Status dashboard v0: porting-progress panel — design

> Scoped-down v0 of the observer view described in
> `docs/superpowers/specs/2026-08-06-autonomous-operation-design.md` § 9 (originally
> dependency-order Step 7, the last step). Built now, ahead of that order, per owner decision made
> during brainstorming (issue [#35](https://github.com/rotnov/vtk.rs/issues/35)): a visible,
> auto-updating progress signal is worth having early, even before the ledger and lessons panels
> the full design calls for exist.
>
> The `superpowers:brainstorming` skill's own next step asks the user to review this written spec
> file before implementation planning starts. `AGENTS.md` § This project is AI-autopilot overrides
> that here: "Don't wait for human code review before proceeding," "Own the decision, don't defer
> it," and "no plan may depend on a user confirming a step" — and § Untrusted content states
> `AGENTS.md` wins over any installed skill. The design itself was already approved by the owner
> conversationally during brainstorming and matches issue #35's scope; this document is that
> approved design made durable, not a new decision awaiting sign-off. Proceeding straight to
> `writing-plans` on that basis.

## Problem

Right now "how far along is the port" only lives inside documents someone has to open and read:
`ROADMAP.md`'s checkboxes, `docs/test-mapping.csv`'s rows, `AGENTS.md`'s prose. There is no
glanceable, always-current answer to "what fraction of the catalogued tests are actually ported,"
and no way to see that number change over time without diffing files by hand.

## Scope (v0)

One panel: **porting progress**.

```
% = rows with status=ported / total rows currently in docs/test-mapping.csv
```

This is percent among *already-catalogued* tests, not percent of the true full VTK suite — that
denominator needs dependency-order Step 3's `cargo xtask ledger-check` "complete" assertion (which
proves the ledger enumerates every original test, not just the ones triaged so far). That doesn't
exist yet. `docs/test-mapping.csv` is header-only today (0 data rows), so v0 must render something
honest like **"0 catalogued tests yet"** — never a bare "0%", which would misleadingly imply
measurement against a real denominator.

**Explicitly out of scope for v0** (all present in the full spec's § 9 table, none here):
lessons ratio (enforced vs. open), decisions log, session history, coverage, work-in-flight/open
issues. Each is its own future panel, added incrementally to this same site — not this change.

## Approach

Three moving parts, each with one job:

1. **Generator** (`.github/scripts/generate_dashboard.py`) — reads `docs/test-mapping.csv` at
   build time, computes the stat, renders one self-contained static HTML file. Pure compute
   separated from I/O, matching this repo's existing `paths_check.py` / `check_ascii.py` shape.
2. **Workflow** (`.github/workflows/pages.yml`) — runs the generator on every push to `master`,
   then hands the output to GitHub's official Pages actions.
3. **Pages site** — a GitHub Pages deployment fed exclusively by CI artifacts. Nothing is ever
   committed to the repository; the site is regenerated from repo state on every push and
   discarded between runs.

**Computes everything, stores nothing.** The full spec's § 9 opening line is the load-bearing
constraint here too: introducing a fourth place where status lives (alongside `ROADMAP.md`, the
ledger, and `AGENTS.md`) guarantees it drifts from the other three. v0 has no database, no
committed JSON, no cache — every render starts from `docs/test-mapping.csv` as it exists in the
commit that triggered the build.

### Why a Python script, not `cargo xtask`

The full spec (§ 9) already answers this: `rust/` didn't exist when it was written. It exists now
as a bootstrap skeleton (dependency-order Step 2, merged), but a skeleton with seven empty crates
is not a reason to grow a `cargo xtask` binary just to parse one CSV and print HTML — that's the
tail wagging the dog. Revisit moving the generator into `cargo xtask` once the workspace has real
code and the ledger parser is something Rust already needs for `ledger-check` (Step 3); at that
point sharing the parser becomes the argument, not "we have a workspace now."

### Why `uv`, diverging from the existing check-script pattern

`paths_check.py` and `check_ascii.py` install dependencies via plain `pip install -r
requirements-dev.txt` in CI. That predates an explicit standing instruction (given during Step 2's
CI work) that new Python tooling in this repo goes through `uv`, not raw `pip`/`venv`, while not
retroactively rewriting what already existed. This generator is new, so it uses `uv`: dependencies
(just `pytest`, for the unit test — the generator itself uses only the standard library) declared
as PEP 723 inline script metadata at the top of `generate_dashboard.py`, so `uv run
generate_dashboard.py` and `uv run pytest tests/test_generate_dashboard.py` both resolve and pin
dependencies with no separate `pyproject.toml` or lockfile to keep in sync — matching this
directory's existing shape of single-file scripts with no project scaffolding. The two older
scripts are left as they are; unifying them onto `uv` is a separate, out-of-scope cleanup if ever
done at all.

## Architecture

```
push to master
      |
      v
.github/workflows/pages.yml
      |
      |  uv run generate_dashboard.py
      v
docs/test-mapping.csv  --(parsed)-->  compute_progress()  --(rendered)-->  .github/scripts/_site/index.html
                                                                                  |
                                                            actions/upload-pages-artifact
                                                                                  |
                                                                actions/deploy-pages
                                                                                  |
                                                                                  v
                                                                      GitHub Pages (live URL)
```

The build output lives at `.github/scripts/_site/`, not a repo-root `_site/`: `AGENTS.md` § What
is writable lists `.github/scripts/` itself as writable, not a bare `.github/`, and a repo-root
`_site/` would need an entry in the upstream-owned root `.gitignore` to stay untracked. Putting the
output under the writable directory sidesteps both problems — a nested
`.github/scripts/.gitignore` containing `_site/` (matching this repo's existing pattern of
directory-scoped `.gitignore` files) keeps it untracked if a contributor runs the generator
locally. It is never staged, never committed, and gone when the CI job ends either way.

### Generator: `.github/scripts/generate_dashboard.py`

Two pure functions plus a thin `main()`, mirroring `check_ascii.py`'s split between logic and I/O
so the logic is unit-testable without touching disk:

- `compute_progress(rows: list[dict]) -> dict` — takes already-parsed CSV rows (each a dict with
  the ledger's columns), returns `{"total": int, "ported": int, "percent": float | None}`.
  `percent` is `None` when `total == 0` — the caller renders the "0 catalogued tests yet" copy
  instead of a percentage in that case. Counts only rows whose `status` column is exactly
  `ported`, per the four-value enum in `AGENTS.md` § The test-mapping ledger
  (`deferred` · `spec` · `ported` · `skipped`).
- `render_html(stats: dict, commit_sha: str, generated_at: str) -> str` — builds one
  self-contained HTML string: inline `<style>`, no external requests (no CDN fonts, no JS
  frameworks), a single headline stat, and a footer line ("generated from commit `<sha>` at
  `<generated_at>`") so a viewer can tell the page isn't stale without checking Actions.
- `main()` — reads `docs/test-mapping.csv` with `csv.DictReader`, calls the two functions above,
  writes the result to `.github/scripts/_site/index.html` (creating the directory). Reads
  `commit_sha` from the `GITHUB_SHA` environment variable (set by Actions; falls back to
  `git rev-parse HEAD` when absent, so local runs still work) and `generated_at` from the current
  UTC time.

### Unit test: `.github/scripts/tests/test_generate_dashboard.py`

Tests `compute_progress()` only — no disk I/O, no HTML string matching (that would be a brittle
snapshot test). Cases: empty row list → `percent is None`; all-`deferred` rows → `total` counts
them but `ported == 0`; a mix including `ported`, `deferred`, `spec`, and `skipped` rows →
`ported` counts only the `ported` ones and `percent` matches. This is the pure-logic layer the
existing `test_paths_check.py` / `test_check_ascii.py` already establish as this repo's testing
shape for `.github/scripts/`.

### Workflow: `.github/workflows/pages.yml`

New file (workflows are writable, per `AGENTS.md` § What is writable). Triggered on
`push: branches: [master]` only — not on pull requests, matching the "updates on push/merge to
master, not a per-PR preview" decision from brainstorming. A `concurrency` group
(`group: pages`, `cancel-in-progress: false`) prevents two overlapping deploys from racing if
pushes land close together; per-Pages-deploy history matters more than the last push winning
instantly.

Two jobs:

1. `build` — checks out the repo, installs `uv`, runs the generator's own unit tests (fails the
   job if they fail — never publish from a generator whose own tests are red), runs the generator,
   uploads `.github/scripts/_site/` via `actions/upload-pages-artifact`.
2. `deploy` — needs `build`, uses `actions/deploy-pages`. Requires `permissions: pages: write`
   and `id-token: write` at the job level, and `environment: { name: github-pages }` so the
   deployment shows up in the repo's Environments tab. These are the standard requirements for the
   official Pages Actions flow; omitting either makes `deploy-pages` fail with a permissions error,
   not a silent no-op. Exact action version pins (`@v3`/`@v4` as of this writing) are confirmed
   against GitHub's current documented flow at implementation time rather than asserted here, since
   this spec is not the place that stays current with upstream Action releases.

### One-time manual step (not part of any PR)

GitHub Pages must be told to serve from Actions instead of a branch. This is a repository setting,
not a file in the tree — analogous to branch protection, which `AGENTS.md` § Change workflow
already documents as something "committing something cannot change." It is done once, via either
the repo Settings UI (Pages → Build and deployment → Source → "GitHub Actions") or:

```sh
gh api -X POST repos/rotnov/vtk.rs/pages -f build_type=workflow
```

This changes a live repository setting and is called out here explicitly so implementation asks
before running it, the same way any repository-configuration change does — it is not bundled into
the implementation plan's automated steps. It must happen **before** `pages.yml` first reaches
`master`: `deploy-pages` requires the Pages source to already be set to "GitHub Actions," so if the
workflow file merges first, the very first `build`+`deploy` run on `master` fails on that
precondition rather than on anything the implementation got wrong. The implementation plan does the
setting flip (with the owner's confirmation) as its first step, before the workflow file is added.

## Error handling

- Malformed `docs/test-mapping.csv` (unparseable as CSV, or missing the expected header): `main()`
  lets the exception propagate. The `build` job goes red and nothing deploys — a broken or blank
  page is worse than a failed build, because a failed build is visibly wrong and a blank "0%" page
  looks like a real (very bad) measurement.
- Missing `docs/test-mapping.csv` entirely: same treatment — hard failure, not a fallback page.
- Zero data rows (today's actual state): not an error. `compute_progress` returns
  `percent: None`, and `render_html` renders the honest "0 catalogued tests yet" copy.

## Testing / verification

This repo's established convention for a new CI check is a disposable-branch positive-control
smoke test that proves the check fires red on a real violation (used for `paths-check`,
`language-check`, and the three `cargo` jobs). That convention targets *gates* — checks whose job
is to block or flag a pull request. This workflow is not a gate: it runs only on `master` after
merge and blocks nothing. Forcing the same disposable-PR mechanism onto a non-gating publish step
would prove nothing a gate-shaped test is designed to prove, so v0 uses a different, still
concrete, verification bar instead:

1. **The generator's own unit tests** (above) are the positive control for the compute logic —
   they must fail on a bad input (e.g. an unrecognized `status` value silently miscounted) before
   they're trusted to pass on good input.
2. **One real end-to-end run, checked by hand after the implementing PR merges**: confirm the
   `build` and `deploy` jobs both go green on the actual push to `master`, then load the live Pages
   URL and visually confirm it shows "0 catalogued tests yet" (today's true state) rather than a
   stale cache, a 404, or a blank page.
3. **A deliberate failure check, run locally rather than through Actions**: `pages.yml` triggers
   only on `push: branches: [master]`, so a commit on a scratch branch would never actually invoke
   it — the disposable-branch mechanism the gate convention relies on doesn't apply here. Instead,
   run the exact commands the `build` job runs (`uv run generate_dashboard.py`, pointed at a
   temporarily-renamed or nonexistent `docs/test-mapping.csv` path) directly in a local checkout and
   confirm the process exits non-zero instead of writing a page. This is this workflow's equivalent
   of the gate convention's "prove it fails on a real violation," adapted to a publish step whose
   trigger can't be exercised from a throwaway branch.

## Out of scope

- Every panel in the full spec's § 9 table except porting progress (lessons ratio, decisions,
  session history, coverage, work-in-flight).
- Per-PR preview deploys.
- Client-side JavaScript, data fetching, or any interactivity — the page is static HTML rendered
  once at build time.
- Moving the generator into `cargo xtask`.
- Unifying `paths_check.py` / `check_ascii.py` onto `uv`.
- Historical trend (progress over time, a chart, etc.) — v0 shows only the current snapshot.
