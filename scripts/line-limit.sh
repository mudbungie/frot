#!/usr/bin/env bash
# Single home for frot's source-file line cap.
#
# Reads newline-delimited file paths on stdin and exits non-zero if any
# exceeds the limit. The number lives here and nowhere else, and both callers
# feed it the same set — `git ls-files '*.rs'`, every tracked source file — so
# the pre-commit hook and CI cannot disagree about what passes.
set -euo pipefail

LIMIT=300
fail=0

while IFS= read -r f; do
  [ -n "$f" ] || continue
  [ -f "$f" ] || continue
  lines=$(wc -l < "$f")
  if [ "$lines" -gt "$LIMIT" ]; then
    echo "line-limit: $f has $lines lines (limit $LIMIT)" >&2
    fail=1
  fi
done

exit "$fail"
