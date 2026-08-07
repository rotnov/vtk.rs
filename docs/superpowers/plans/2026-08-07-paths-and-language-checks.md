# CI: paths-check and language-check Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Stand up the first two CI checks from the autonomy spec's dependency order —
`paths-check` (fails a PR that touches anything outside AGENTS.md's writable allowlist) and
`language-check` (fails a PR that adds non-English text to `docs/`, `rust/`, or the root
meta-files) — so two of AGENTS.md's prose rules become machine-enforced instead of advisory.

**Architecture:** Two small, pure, unit-testable Python scripts under `.github/scripts/`, each
wrapped by a thin CLI `main()`, invoked from one GitHub Actions workflow with two jobs. No `rust/`
workspace exists yet, so neither script may assume `cargo`/`rustc` is available. The scripts stay
runnable and testable with only the Python standard library plus `pytest`.

**Tech Stack:** Python 3.12 (stdlib only for the scripts; `pytest` for their own tests), GitHub
Actions (`ubuntu-latest`, `actions/checkout@v4`, `actions/setup-python@v5`).

## Global Constraints

- Everything committed to this repository is written in English — code, comments, docs, commit
  messages (AGENTS.md § Language).
- Every change lands through a pull request; no direct pushes to `master` (AGENTS.md § Change
  workflow).
- Writable today: `rust/`, `docs/`, `.claude/`, `.github/workflows/`, and the root meta-files
  `AGENTS.md`, `ROADMAP.md`, `CLAUDE.md` (AGENTS.md § What is writable). This plan adds
  `.github/scripts/` to that list in Task 1 — upstream VTK's own `.github/` contains only
  `.github/pull_request_template.md` (verified: `git ls-tree -r --name-only v9.6.2 -- .github`),
  so a `scripts/` subdirectory there cannot collide with anything upstream.
- `rust/` does not exist yet. Both scripts and both CI jobs must work today, before it does.
- Per the autonomy spec's dependency order (`docs/superpowers/specs/2026-08-06-autonomous-operation-design.md`
  § Dependency order, step 1), these checks are built now but are **not** made required in branch
  protection yet — that is step 4, after the `rust/` workspace and `ledger-check` exist too, so
  all required checks are added together.
- The upstream-sync merge (ADR `docs/decisions/0003-upstream-sync-strategy.md`) legitimately
  touches every upstream path at once. `paths-check` must not treat that as a violation; it is
  exempted via a `upstream-sync` label on that PR, not via weakening the allowlist.

## File Structure

- `.github/scripts/paths_check.py` — pure allowlist logic + a `git diff`-driven CLI. One
  responsibility: is this set of changed paths inside AGENTS.md's writable allowlist.
- `.github/scripts/check_ascii.py` — pure text-scanning logic + a filesystem-walking CLI. One
  responsibility: does this file contain a non-English-script character.
- `.github/scripts/requirements-dev.txt` — pins `pytest` for both scripts' own test suites.
- `.github/scripts/tests/test_paths_check.py`, `.github/scripts/tests/test_check_ascii.py` — each
  script's own tests live next to it, not in a repo-wide `tests/` folder, since nothing else in
  the repo shares this Python code yet.
- `.github/workflows/repo-checks.yml` — one workflow, two jobs (`paths-check`, `language-check`),
  each job runs its script's own tests before running the check for real (a broken check script
  must fail the job, not silently pass).
- `AGENTS.md` — extend § What is writable to cover `.github/scripts/`; extend § Required checks
  to list the two new checks and their current (not-yet-required) status.

## Task 1: Extend the writable allowlist to cover `.github/scripts/`

**Files:**
- Modify: `AGENTS.md:237-244` (§ What is writable)

**Interfaces:**
- Produces: the allowlist text that Task 2's `ALLOWED_DIRS` must match exactly, so the code and
  the doc it enforces never quietly diverge.

- [ ] **Step 1: Edit the "What is writable" section**

Current text (`AGENTS.md:239-244`):

```
Writable: `rust/`, `docs/`, `.claude/` (agent tooling — installed skills, settings),
`.github/workflows/` (our CI; nothing upstream lives there), and the project meta-files at the
repository root that are *not* part of upstream VTK — currently
`AGENTS.md`, `ROADMAP.md`, `CLAUDE.md`. (Verify with
`git ls-tree --name-only <upstream-commit>`: if a root file exists in the upstream tree, it is
not ours to touch.)
```

Replace with:

```
Writable: `rust/`, `docs/`, `.claude/` (agent tooling — installed skills, settings),
`.github/workflows/` and `.github/scripts/` (our CI and the scripts it runs — upstream's own
`.github/` holds only `pull_request_template.md`, so nothing else under it is contested), and the
project meta-files at the repository root that are *not* part of upstream VTK — currently
`AGENTS.md`, `ROADMAP.md`, `CLAUDE.md`. (Verify with
`git ls-tree --name-only <upstream-commit>`: if a root file exists in the upstream tree, it is
not ours to touch.)
```

- [ ] **Step 2: Commit**

```bash
git add AGENTS.md
git commit -m "docs: allow .github/scripts/ as writable, next to .github/workflows/"
```

## Task 2: `paths_check.py` — pure allowlist logic

**Files:**
- Create: `.github/scripts/paths_check.py`
- Test: `.github/scripts/tests/test_paths_check.py`

**Interfaces:**
- Produces: `is_writable(path: str) -> bool`, `find_violations(changed_files: list[str]) -> list[str]`
  — both pure, no I/O. Task 3 adds the git-calling and CLI code around these in the same file.

- [ ] **Step 1: Write the failing tests**

```python
# .github/scripts/tests/test_paths_check.py
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from paths_check import is_writable, find_violations


def test_is_writable_allows_docs():
    assert is_writable("docs/decisions/0004-x.md") is True


def test_is_writable_allows_scripts_and_workflows():
    assert is_writable(".github/scripts/paths_check.py") is True
    assert is_writable(".github/workflows/repo-checks.yml") is True


def test_is_writable_allows_root_meta_files():
    assert is_writable("AGENTS.md") is True
    assert is_writable("ROADMAP.md") is True
    assert is_writable("CLAUDE.md") is True


def test_is_writable_rejects_upstream_source():
    assert is_writable("Common/Core/vtkObject.cxx") is False


def test_is_writable_rejects_root_files_that_are_not_ours():
    assert is_writable("CMakeLists.txt") is False
    assert is_writable("README.md") is False


def test_is_writable_rejects_lookalike_prefix():
    # "docs-old/x" must not match the "docs/" allowlist entry by accident.
    assert is_writable("docs-old/x.md") is False


def test_find_violations_returns_only_disallowed_paths():
    changed = [
        "docs/x.md",
        "Common/Core/vtkObject.cxx",
        "rust/Cargo.toml",
        "Testing/Cxx/foo.cxx",
    ]
    assert find_violations(changed) == [
        "Common/Core/vtkObject.cxx",
        "Testing/Cxx/foo.cxx",
    ]


def test_find_violations_empty_when_all_writable():
    assert find_violations(["docs/x.md", "AGENTS.md"]) == []
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `python3 -m pytest .github/scripts/tests/test_paths_check.py -v`
Expected: `ModuleNotFoundError: No module named 'paths_check'`

- [ ] **Step 3: Write the minimal implementation**

```python
# .github/scripts/paths_check.py
"""Fail a PR that touches paths outside AGENTS.md's writable allowlist.

See AGENTS.md's "What is writable" section. The allowlist here must match that section
exactly, or the check and the doc it enforces will quietly drift.
"""

ALLOWED_DIRS = [
    "rust/",
    "docs/",
    ".claude/",
    ".github/workflows/",
    ".github/scripts/",
]

ALLOWED_FILES = {
    "AGENTS.md",
    "ROADMAP.md",
    "CLAUDE.md",
}


def is_writable(path):
    if path in ALLOWED_FILES:
        return True
    return any(path.startswith(prefix) for prefix in ALLOWED_DIRS)


def find_violations(changed_files):
    return [path for path in changed_files if not is_writable(path)]
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `python3 -m pytest .github/scripts/tests/test_paths_check.py -v`
Expected: 7 passed

- [ ] **Step 5: Commit**

```bash
git add .github/scripts/paths_check.py .github/scripts/tests/test_paths_check.py
git commit -m "ci: add paths_check.py allowlist logic with tests"
```

## Task 3: `paths_check.py` — git integration and CLI

**Files:**
- Modify: `.github/scripts/paths_check.py`
- Test: `.github/scripts/tests/test_paths_check.py`

**Interfaces:**
- Consumes: `is_writable`, `find_violations` from Task 2 (same file, no import needed).
- Produces: `get_changed_files(base_ref: str, head_ref: str, cwd=None) -> list[str]` and
  `main(argv: list[str]) -> int`. `main` is what the workflow (Task 6) invokes via
  `python3 .github/scripts/paths_check.py <base_ref> <head_ref> <skip:true|false>`.

- [ ] **Step 1: Write the failing test**

```python
# append to .github/scripts/tests/test_paths_check.py
import subprocess

from paths_check import get_changed_files, main


def _git(*args, cwd):
    subprocess.run(["git", *args], cwd=cwd, check=True, capture_output=True, text=True)


def _rev_parse(ref, cwd):
    return subprocess.run(
        ["git", "rev-parse", ref], cwd=cwd, check=True, capture_output=True, text=True
    ).stdout.strip()


def _make_repo_with_one_upstream_touch(tmp_path):
    repo = tmp_path / "repo"
    repo.mkdir()
    _git("init", "-q", cwd=repo)
    _git("config", "user.email", "test@example.com", cwd=repo)
    _git("config", "user.name", "Test", cwd=repo)
    (repo / "AGENTS.md").write_text("v1\n")
    _git("add", "AGENTS.md", cwd=repo)
    _git("commit", "-q", "-m", "base", cwd=repo)
    base = _rev_parse("HEAD", cwd=repo)

    (repo / "Common").mkdir()
    (repo / "Common" / "vtkObject.cxx").write_text("// upstream\n")
    (repo / "docs").mkdir()
    (repo / "docs" / "note.md").write_text("note\n")
    _git("add", "-A", cwd=repo)
    _git("commit", "-q", "-m", "touch upstream and docs", cwd=repo)
    head = _rev_parse("HEAD", cwd=repo)
    return repo, base, head


def test_get_changed_files_reads_a_real_diff(tmp_path):
    repo, base, head = _make_repo_with_one_upstream_touch(tmp_path)
    changed = get_changed_files(base, head, cwd=repo)
    assert sorted(changed) == ["Common/vtkObject.cxx", "docs/note.md"]


def test_main_fails_on_violation(tmp_path, capsys):
    repo, base, head = _make_repo_with_one_upstream_touch(tmp_path)
    exit_code = main(["paths_check.py", base, head, "false"], cwd=repo)
    assert exit_code == 1
    assert "Common/vtkObject.cxx" in capsys.readouterr().out


def test_main_passes_when_all_writable(tmp_path, capsys):
    repo = tmp_path / "repo"
    repo.mkdir()
    _git("init", "-q", cwd=repo)
    _git("config", "user.email", "test@example.com", cwd=repo)
    _git("config", "user.name", "Test", cwd=repo)
    (repo / "AGENTS.md").write_text("v1\n")
    _git("add", "AGENTS.md", cwd=repo)
    _git("commit", "-q", "-m", "base", cwd=repo)
    base = _rev_parse("HEAD", cwd=repo)
    (repo / "docs").mkdir()
    (repo / "docs" / "note.md").write_text("note\n")
    _git("add", "-A", cwd=repo)
    _git("commit", "-q", "-m", "docs only", cwd=repo)
    head = _rev_parse("HEAD", cwd=repo)

    exit_code = main(["paths_check.py", base, head, "false"], cwd=repo)
    assert exit_code == 0


def test_main_skips_when_upstream_sync_label_present(tmp_path, capsys):
    repo, base, head = _make_repo_with_one_upstream_touch(tmp_path)
    exit_code = main(["paths_check.py", base, head, "true"], cwd=repo)
    assert exit_code == 0
    assert "skipped" in capsys.readouterr().out
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `python3 -m pytest .github/scripts/tests/test_paths_check.py -v`
Expected: `ImportError: cannot import name 'get_changed_files'` (and `'main'`)

- [ ] **Step 3: Write the minimal implementation**

```python
# append to .github/scripts/paths_check.py
import subprocess
import sys


def get_changed_files(base_ref, head_ref, cwd=None):
    result = subprocess.run(
        ["git", "diff", "--no-renames", "--name-only", base_ref, head_ref],
        capture_output=True,
        text=True,
        check=True,
        cwd=cwd,
    )
    return [line for line in result.stdout.splitlines() if line]


def main(argv, cwd=None):
    if len(argv) != 4:
        print("usage: paths_check.py <base_ref> <head_ref> <skip:true|false>", file=sys.stderr)
        return 2
    _, base_ref, head_ref, skip = argv
    if skip.strip().lower() == "true":
        print(
            "paths-check: skipped (PR labeled upstream-sync; see "
            "docs/decisions/0003-upstream-sync-strategy.md)"
        )
        return 0
    changed = get_changed_files(base_ref, head_ref, cwd=cwd)
    violations = find_violations(changed)
    if violations:
        print("paths-check: FAILED. These paths are outside the writable allowlist:")
        for path in violations:
            print(f"  - {path}")
        print("See AGENTS.md's \"What is writable\" section.")
        return 1
    print(f"paths-check: OK ({len(changed)} changed file(s), all writable)")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `python3 -m pytest .github/scripts/tests/test_paths_check.py -v`
Expected: 11 passed

- [ ] **Step 5: Commit**

```bash
git add .github/scripts/paths_check.py .github/scripts/tests/test_paths_check.py
git commit -m "ci: add paths_check.py git diff integration, CLI, and upstream-sync skip"
```

## Task 4: `check_ascii.py` — pure text-scanning logic

**Files:**
- Create: `.github/scripts/check_ascii.py`
- Test: `.github/scripts/tests/test_check_ascii.py`

**Interfaces:**
- Produces: `find_violations_in_text(text: str) -> list[tuple[int, int, str]]` (1-indexed line,
  1-indexed column, offending character), and the module-level `ALLOWED_NON_ASCII` set that
  Task 5's file walker reuses.

The allowlist below is not arbitrary — it is every non-ASCII character actually present in this
repo's prose today, found by scanning `AGENTS.md`, `ROADMAP.md`, `CLAUDE.md`, and `docs/**/*.md`:
em dash (196 uses), section sign (17), rightwards arrow (13), middle dot (7), en dash (2). Nothing
else appears. Anything outside this set — Cyrillic included — is a violation.

- [ ] **Step 1: Write the failing tests**

```python
# .github/scripts/tests/test_check_ascii.py
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from check_ascii import find_violations_in_text


def test_pure_ascii_has_no_violations():
    assert find_violations_in_text("Just English text.\n") == []


def test_allowed_typographic_chars_pass():
    text = "one — two – three → four § 5 · six\n"
    assert find_violations_in_text(text) == []


def test_cyrillic_is_flagged():
    violations = find_violations_in_text("word привет end\n")
    assert len(violations) == 6
    assert all(ch in "привет" for _, _, ch in violations)


def test_line_and_column_are_1_indexed():
    text = "ok\nбad\n"
    assert find_violations_in_text(text) == [(2, 1, "б")]


def test_accented_latin_is_also_flagged():
    # Not in the allowlist derived from real usage, so it is out of scope too -
    # a real occurrence should be added to ALLOWED_NON_ASCII deliberately, not silently pass.
    assert find_violations_in_text("café\n") == [(1, 4, "é")]
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `python3 -m pytest .github/scripts/tests/test_check_ascii.py -v`
Expected: `ModuleNotFoundError: No module named 'check_ascii'`

- [ ] **Step 3: Write the minimal implementation**

```python
# .github/scripts/check_ascii.py
"""Fail on non-English-script characters in docs/, rust/, and the root meta-files.

See AGENTS.md's Language section: everything committed is English. A small set of Latin
typographic characters already used throughout this repo's prose is allowed; anything else
above ASCII (Cyrillic, CJK, accented Latin, etc.) is flagged.
"""

ALLOWED_NON_ASCII = {
    "—",  # em dash
    "–",  # en dash
    "§",  # section sign
    "→",  # rightwards arrow
    "·",  # middle dot
}


def find_violations_in_text(text):
    violations = []
    for lineno, line in enumerate(text.splitlines(), start=1):
        for col, ch in enumerate(line, start=1):
            if ord(ch) > 127 and ch not in ALLOWED_NON_ASCII:
                violations.append((lineno, col, ch))
    return violations
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `python3 -m pytest .github/scripts/tests/test_check_ascii.py -v`
Expected: 5 passed

- [ ] **Step 5: Commit**

```bash
git add .github/scripts/check_ascii.py .github/scripts/tests/test_check_ascii.py
git commit -m "ci: add check_ascii.py scanning logic with tests"
```

## Task 5: `check_ascii.py` — file walking and CLI

**Files:**
- Modify: `.github/scripts/check_ascii.py`
- Test: `.github/scripts/tests/test_check_ascii.py`

**Interfaces:**
- Consumes: `find_violations_in_text`, `ALLOWED_NON_ASCII` from Task 4 (same file).
- Produces: `iter_scan_files(root: Path = Path(".")) -> list[Path]` and `main() -> int`. `main` is
  what the workflow (Task 6) invokes via `python3 .github/scripts/check_ascii.py` with no
  arguments, run from the repository root.

- [ ] **Step 1: Write the failing test**

```python
# append to .github/scripts/tests/test_check_ascii.py
from check_ascii import iter_scan_files, main


def test_iter_scan_files_includes_only_root_meta_files_we_own(tmp_path):
    (tmp_path / "AGENTS.md").write_text("x")
    (tmp_path / "ROADMAP.md").write_text("x")
    (tmp_path / "CLAUDE.md").write_text("x")
    (tmp_path / "README.md").write_text("x")  # upstream file, out of scope
    found = {p.name for p in iter_scan_files(tmp_path)}
    assert found == {"AGENTS.md", "ROADMAP.md", "CLAUDE.md"}


def test_iter_scan_files_walks_docs_and_rust_but_not_upstream_dirs(tmp_path):
    (tmp_path / "docs").mkdir()
    (tmp_path / "docs" / "a.md").write_text("x")
    (tmp_path / "rust").mkdir()
    (tmp_path / "rust" / "lib.rs").write_text("x")
    (tmp_path / "Common").mkdir()
    (tmp_path / "Common" / "x.cxx").write_text("x")
    found = {str(p.relative_to(tmp_path)) for p in iter_scan_files(tmp_path)}
    assert found == {"docs/a.md", "rust/lib.rs"}


def test_main_fails_when_a_scanned_file_has_cyrillic(tmp_path, monkeypatch, capsys):
    (tmp_path / "docs").mkdir()
    (tmp_path / "docs" / "a.md").write_text("word привет end\n")
    monkeypatch.chdir(tmp_path)
    assert main() == 1
    assert "docs/a.md" in capsys.readouterr().out.replace("\\", "/")


def test_main_passes_when_scanned_files_are_clean(tmp_path, monkeypatch, capsys):
    (tmp_path / "docs").mkdir()
    (tmp_path / "docs" / "a.md").write_text("all English — clean\n")
    monkeypatch.chdir(tmp_path)
    assert main() == 0
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `python3 -m pytest .github/scripts/tests/test_check_ascii.py -v`
Expected: `ImportError: cannot import name 'iter_scan_files'` (and `'main'`)

- [ ] **Step 3: Write the minimal implementation**

```python
# append to .github/scripts/check_ascii.py
from pathlib import Path

SCAN_DIRS = ["rust", "docs"]
SCAN_FILES = ["AGENTS.md", "ROADMAP.md", "CLAUDE.md"]


def iter_scan_files(root=Path(".")):
    root = Path(root)
    files = []
    for name in SCAN_FILES:
        candidate = root / name
        if candidate.is_file():
            files.append(candidate)
    for dirname in SCAN_DIRS:
        dirpath = root / dirname
        if dirpath.is_dir():
            files.extend(p for p in dirpath.rglob("*") if p.is_file())
    return files


def main():
    failed = False
    for path in iter_scan_files():
        try:
            text = path.read_text(encoding="utf-8")
        except UnicodeDecodeError:
            print(f"language-check: FAILED. {path} is not valid UTF-8.")
            failed = True
            continue
        violations = find_violations_in_text(text)
        if violations:
            failed = True
            print(f"language-check: FAILED. {path} has non-English characters:")
            for lineno, col, ch in violations:
                print(f"  - line {lineno}, col {col}: {ch!r} (U+{ord(ch):04X})")
    if failed:
        print("See AGENTS.md's Language section. Everything committed must be English.")
        return 1
    print("language-check: OK")
    return 0


if __name__ == "__main__":
    import sys

    sys.exit(main())
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `python3 -m pytest .github/scripts/tests/test_check_ascii.py -v`
Expected: 9 passed

- [ ] **Step 5: Run both scripts against the real repository**

Run: `python3 .github/scripts/check_ascii.py`
Expected: `language-check: OK` — this repo's real `docs/`, `AGENTS.md`, `ROADMAP.md`, `CLAUDE.md`
must already pass, since `ALLOWED_NON_ASCII` was derived from what they actually contain.

- [ ] **Step 6: Commit**

```bash
git add .github/scripts/check_ascii.py .github/scripts/tests/test_check_ascii.py
git commit -m "ci: add check_ascii.py file walking, CLI, and root meta-file scope"
```

## Task 6: Wire both scripts into a GitHub Actions workflow

**Files:**
- Create: `.github/workflows/repo-checks.yml`
- Create: `.github/scripts/requirements-dev.txt`

**Interfaces:**
- Consumes: `paths_check.main` and `check_ascii.main` from Tasks 3 and 5, invoked as CLIs exactly
  as those tasks specified.

- [ ] **Step 1: Add the pytest pin**

```
# .github/scripts/requirements-dev.txt
pytest>=8,<9
```

- [ ] **Step 2: Write the workflow**

```yaml
# .github/workflows/repo-checks.yml
name: repo-checks

on:
  pull_request:
    branches: [master]

jobs:
  paths-check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0

      - uses: actions/setup-python@v5
        with:
          python-version: '3.12'

      - name: Install test dependencies
        run: pip install -r .github/scripts/requirements-dev.txt

      - name: Run paths_check.py's own tests
        run: python3 -m pytest .github/scripts/tests/test_paths_check.py -v

      - name: Fetch base branch
        run: git fetch origin "${{ github.base_ref }}"

      - name: Check changed paths against the writable allowlist
        env:
          UPSTREAM_SYNC: ${{ contains(github.event.pull_request.labels.*.name, 'upstream-sync') }}
        run: |
          BASE=$(git merge-base "origin/${{ github.base_ref }}" HEAD)
          python3 .github/scripts/paths_check.py "$BASE" HEAD "$UPSTREAM_SYNC"

  language-check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - uses: actions/setup-python@v5
        with:
          python-version: '3.12'

      - name: Install test dependencies
        run: pip install -r .github/scripts/requirements-dev.txt

      - name: Run check_ascii.py's own tests
        run: python3 -m pytest .github/scripts/tests/test_check_ascii.py -v

      - name: Check for non-English content
        run: python3 .github/scripts/check_ascii.py
```

Two notes for whoever edits this later:

- `HEAD` is used deliberately instead of `${{ github.sha }}`: on `pull_request` events
  `github.sha` is the synthetic merge commit GitHub builds for the PR, and `actions/checkout@v4`
  already checks that merge commit out as `HEAD` — passing the same ref twice under two different
  names invites the two to drift if the checkout behavior ever changes.
- `paths-check` needs `fetch-depth: 0` (full history, to compute a real merge-base);
  `language-check` only reads the working tree at `HEAD`, so it doesn't.

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/repo-checks.yml .github/scripts/requirements-dev.txt
git commit -m "ci: wire paths_check.py and check_ascii.py into repo-checks.yml"
```

## Task 7: Update AGENTS.md's Required checks list, and verify the workflow live

**Files:**
- Modify: `AGENTS.md:153-155`, `AGENTS.md:164-173` (§ Change workflow / § Required checks)

**Interfaces:**
- None — this task documents Tasks 1-6's result and proves it works against a real PR; it
  produces nothing later tasks consume.

- [ ] **Step 1: Update the "not configured yet" paragraph**

Current text (`AGENTS.md:153-155`):

```
Required status checks are **not** configured yet — there is no CI to require, because `rust/`
does not exist. Wire them up with the workflow (Phase 0), or the "green CI is the review" rule
above is an honour system.
```

Replace with:

```
`paths-check` and `language-check` run on every PR today (see § Required checks) but are **not**
yet marked required in branch protection — that happens once the `rust/` workspace and
`cargo xtask ledger-check` exist too, so every required check is added in one pass (see
`docs/superpowers/specs/2026-08-06-autonomous-operation-design.md` § Dependency order). Until
then, a red `paths-check` or `language-check` is a signal to fix before merging, not a gate that
blocks the merge button.
```

- [ ] **Step 2: Add the two checks to the "Required checks" list**

Current text (`AGENTS.md:164-173`):

```
### Required checks

- `cargo test --workspace --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo fmt --all --check`
- `cargo check --target wasm32-unknown-unknown` for `Common*`/`Filters*` — see **WebAssembly**
- `cargo xtask ledger-check` — the three ledger assertions (*exists*, *complete*, *fresh*); see
  **The test-mapping ledger**. Cheap, and it fails loudly the moment the ledger stops describing
  the reference tree instead of letting it drift
- the coverage gate, below
```

Replace with:

```
### Required checks

- `paths-check` — every changed path in the PR is inside § What is writable, or the PR carries
  the `upstream-sync` label (`.github/scripts/paths_check.py`). Live today.
- `language-check` — no non-English-script character in `docs/`, `rust/`, or the root meta-files
  (`.github/scripts/check_ascii.py`). Live today.
- `cargo test --workspace --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo fmt --all --check`
- `cargo check --target wasm32-unknown-unknown` for `Common*`/`Filters*` — see **WebAssembly**
- `cargo xtask ledger-check` — the three ledger assertions (*exists*, *complete*, *fresh*); see
  **The test-mapping ledger**. Cheap, and it fails loudly the moment the ledger stops describing
  the reference tree instead of letting it drift
- the coverage gate, below
```

- [ ] **Step 3: Commit**

```bash
git add AGENTS.md
git commit -m "docs: record paths-check and language-check as live, not yet required"
```

- [ ] **Step 4: Open the PR for this whole plan and confirm both checks run and pass**

```bash
git push -u origin HEAD
gh pr create --title "Add paths-check and language-check CI workflows" --body \
  "Implements dependency-order step 1 of docs/superpowers/specs/2026-08-06-autonomous-operation-design.md.

Adds two GitHub Actions checks: paths-check (fails a PR touching anything outside AGENTS.md's
writable allowlist) and language-check (fails a PR adding non-English-script text to docs/,
rust/, or the root meta-files). Neither is yet required in branch protection - see the updated
AGENTS.md paragraph for why.

Both scripts are pure-logic-plus-thin-CLI and carry their own pytest suites, run as the first
step of each CI job."
```

Expected: this PR only touches `.github/`, `AGENTS.md` — all writable — so `paths-check` should
report `paths-check: OK`, and `language-check` should report `language-check: OK`. Watch the
checks with `gh pr checks --watch` and confirm both are green before merging.

- [ ] **Step 5: Prove paths-check actually fails on a real violation, then discard the proof**

This is the one behavior that cannot be verified by a script's own unit tests: does the workflow,
running for real on GitHub, actually turn red for a PR that touches an upstream path. Do this on
a disposable branch, off `master` at its current tip, and never merge it:

```bash
git checkout master
git pull
git checkout -b tmp-paths-check-smoke-test
echo "" >> Common/Core/vtkObject.h
git add Common/Core/vtkObject.h
git commit -m "tmp: smoke-test paths-check (do not merge)"
git push -u origin HEAD
gh pr create --title "tmp: smoke-test paths-check (do not merge)" --body "Verifies paths-check fails on an upstream-path change. Closing without merging."
gh pr checks --watch
```

Expected: `paths-check` fails, listing `Common/Core/vtkObject.h`. Then clean up:

```bash
gh pr close --delete-branch
git checkout master
git branch -D tmp-paths-check-smoke-test
```

- [ ] **Step 6: Merge the real PR from Step 4**

```bash
gh pr merge --squash --delete-branch
```
