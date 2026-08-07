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
