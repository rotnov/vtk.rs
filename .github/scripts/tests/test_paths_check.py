import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from paths_check import is_writable, find_violations, get_changed_files, main


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
