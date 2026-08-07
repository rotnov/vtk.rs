# /// script
# requires-python = ">=3.12"
# dependencies = []
# ///
"""Generate the status-dashboard v0 static page: one porting-progress stat.

Reads docs/test-mapping.csv and renders a single self-contained HTML file to
.github/scripts/_site/index.html. Computes everything from repo state at build time; stores
nothing. See docs/superpowers/specs/2026-08-07-status-dashboard-design.md.
"""

import csv
import os
import subprocess
from datetime import datetime, timezone
from pathlib import Path


def compute_progress(rows):
    total = len(rows)
    ported = sum(1 for row in rows if row.get("status") == "ported")
    percent = (ported / total * 100) if total > 0 else None
    return {"total": total, "ported": ported, "percent": percent}


def render_html(stats, commit_sha, generated_at):
    total = stats["total"]
    ported = stats["ported"]
    percent = stats["percent"]
    if percent is None:
        headline = "0 catalogued tests yet"
    else:
        headline = f"{percent:.1f}% ported ({ported}/{total} catalogued tests)"
    return f"""<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>vtk2rust porting progress</title>
<style>
  body {{
    font-family: system-ui, sans-serif;
    background: #0b0d12;
    color: #e6e6e6;
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100vh;
    margin: 0;
  }}
  main {{ text-align: center; }}
  h1 {{ font-size: 2.5rem; margin: 0 0 0.5rem; font-weight: 600; }}
  footer {{ margin-top: 2rem; color: #888; font-size: 0.85rem; }}
</style>
</head>
<body>
<main>
<h1>{headline}</h1>
<footer>generated from commit {commit_sha} at {generated_at}</footer>
</main>
</body>
</html>
"""


def main():
    csv_path = Path("docs/test-mapping.csv")
    with csv_path.open(newline="", encoding="utf-8") as f:
        rows = list(csv.DictReader(f))

    stats = compute_progress(rows)

    commit_sha = os.environ.get("GITHUB_SHA")
    if not commit_sha:
        commit_sha = subprocess.check_output(
            ["git", "rev-parse", "HEAD"], text=True
        ).strip()

    generated_at = datetime.now(timezone.utc).strftime("%Y-%m-%d %H:%M UTC")

    html = render_html(stats, commit_sha, generated_at)

    out_dir = Path(".github/scripts/_site")
    out_dir.mkdir(parents=True, exist_ok=True)
    out_path = out_dir / "index.html"
    out_path.write_text(html, encoding="utf-8")

    print(f"generate_dashboard: wrote {out_path} ({stats})")


if __name__ == "__main__":
    main()
