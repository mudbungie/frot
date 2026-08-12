#!/usr/bin/env bash
# Single home for frot's source-file line cap.
#
# Reads newline-delimited file paths on stdin and exits non-zero if any
# exceeds the limit. The number lives here and nowhere else; *which files* is
# the separate fact `scripts/source-files.sh` owns, and every caller composes
# the two through `make size`, so the pre-commit hook and CI cannot disagree
# about what passes. Reading paths rather than computing them is what lets
# `tests/hygiene/size.rs` feed this a deliberately-bad file: a limiter with no
# input of its own could not be shown to fail.
#
# This header used to claim its callers fed it "every tracked source file",
# while both spelled out `git ls-files '*.rs'` — so the 21-file JS prelude went
# unchecked and `elem2.js` sat over the cap (bl-6da7). Do not describe a set
# here; name the script that computes it.
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
