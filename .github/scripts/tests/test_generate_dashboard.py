import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from generate_dashboard import compute_progress


def test_compute_progress_empty_rows():
    assert compute_progress([]) == {"total": 0, "ported": 0, "percent": None}


def test_compute_progress_all_deferred():
    rows = [{"status": "deferred"}, {"status": "deferred"}]
    result = compute_progress(rows)
    assert result == {"total": 2, "ported": 0, "percent": 0.0}


def test_compute_progress_mixed_statuses():
    rows = [
        {"status": "ported"},
        {"status": "ported"},
        {"status": "deferred"},
        {"status": "spec"},
        {"status": "skipped"},
    ]
    result = compute_progress(rows)
    assert result == {"total": 5, "ported": 2, "percent": 40.0}


if __name__ == "__main__":
    import pytest

    raise SystemExit(pytest.main([__file__, "-v"]))
