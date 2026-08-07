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
