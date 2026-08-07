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
