#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"

g++ -O2 -g -std=c++17 points_bounds.cpp -o points_bounds

# --collect-atstart=no: nothing is counted until CALLGRIND_TOGGLE_COLLECT turns collection on
# inside main() — so the printed "summary:" line reflects only the bounds_of_points() call, not
# process startup or generate_coords().
valgrind --tool=callgrind --collect-atstart=no --callgrind-out-file=callgrind.out \
  ./points_bounds

echo "--- full annotated output ---"
callgrind_annotate callgrind.out

echo "--- summary line (matches the 'events:' header line's column order) ---"
grep '^events:' callgrind.out
grep '^summary:' callgrind.out
