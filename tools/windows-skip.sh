#!/bin/sh
# Read a Windows list from docs/testing.md.
# Usage: windows-skip.sh <compile|test>
#   compile: packages that are not built (windows-skip)
#   test:    product packages that are built but not tested (windows-no-test)
# One package name per line.
set -eu
kind="${1:?usage: windows-skip.sh compile|test}"
root="$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)"
doc="$root/docs/testing.md"
case "$kind" in
  compile) marker="<!-- windows-skip" ;;
  test) marker="<!-- windows-no-test" ;;
  *)
    echo "usage: windows-skip.sh compile|test" >&2
    exit 2
    ;;
esac
if [ ! -f "$doc" ]; then
  echo "windows list missing: docs/testing.md" >&2
  exit 1
fi
names="$(awk -v marker="$marker" '
  $0 == marker { inside = 1; next }
  $0 == "-->" && inside { inside = 0; next }
  inside && $0 !~ /^[[:space:]]*$/ && $0 !~ /^#/ { print }
' "$doc")"
if [ -z "$names" ]; then
  echo "windows $kind list is empty (docs/testing.md)" >&2
  exit 1
fi
bad="$(printf '%s\n' "$names" | grep -v -E '^[A-Za-z0-9_-]+$' || true)"
if [ -n "$bad" ]; then
  echo "windows $kind list has an unexpected package name" >&2
  exit 1
fi
printf '%s\n' "$names"
