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
