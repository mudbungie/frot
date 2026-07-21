#!/usr/bin/env bash
# Assert a built binary sits inside frot's documented static-binary envelope.
#
# Single home for the 5-15 MiB bounds (VISION.md): the release workflow calls
# this after stripping the musl binary, so a bloat or a broken strip fails the
# release instead of shipping.
set -euo pipefail

bin="${1:?usage: check-size.sh <binary>}"
min=$((5 * 1024 * 1024))
max=$((15 * 1024 * 1024))

size=$(stat -c '%s' "$bin")
if [ "$size" -lt "$min" ] || [ "$size" -gt "$max" ]; then
  echo "check-size: $bin is $size bytes, outside the 5-15 MiB envelope" >&2
  exit 1
fi

echo "check-size: $bin is $size bytes (within 5-15 MiB)"
