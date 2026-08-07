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
