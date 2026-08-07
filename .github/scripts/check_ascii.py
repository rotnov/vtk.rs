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
