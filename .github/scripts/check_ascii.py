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
