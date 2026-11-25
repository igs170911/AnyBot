#!/bin/sh
# Read the Windows skip list from docs/testing.md (the windows-skip block).
# Usage: windows-skip.sh [--cargo]
#   default: one package name per line
#   --cargo: one line of repeated --exclude <name>, for cargo build / nextest
set -eu
mode="${1:-list}"
root="$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)"
doc="$root/docs/testing.md"
if [ ! -f "$doc" ]; then
  echo "windows skip list missing: docs/testing.md" >&2
  exit 1
fi
names="$(awk '
  $0 == "<!-- windows-skip" { inside = 1; next }
  $0 == "-->" && inside { inside = 0; next }
  inside && $0 !~ /^[[:space:]]*$/ && $0 !~ /^#/ { print }
' "$doc")"
if [ -z "$names" ]; then
  echo "windows skip list is empty (docs/testing.md)" >&2
  exit 1
fi
bad="$(printf '%s\n' "$names" | grep -v -E '^[A-Za-z0-9_-]+$' || true)"
if [ -n "$bad" ]; then
  echo "windows skip list has an unexpected package name" >&2
  exit 1
fi
case "$mode" in
  list)
    printf '%s\n' "$names"
    ;;
  --cargo)
    printf '%s\n' "$names" | awk '{ printf "--exclude %s ", $0 }'
    printf '\n'
    ;;
  *)
    echo "usage: windows-skip.sh [--cargo]" >&2
    exit 2
    ;;
esac
