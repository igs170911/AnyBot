#!/bin/sh
# Print one line: tests ran: <N>
# N is nextest "N tests run" from the log, plus pnpm/vitest "Tests N passed".
# Usage: test-count.sh <os-label> <nextest-log>
set -eu
label="${1:?os label}"
log="${2:?nextest log}"
case "$label" in
  *[!A-Za-z0-9_-]*)
    echo "os label has unexpected characters" >&2
    exit 1
    ;;
esac
if [ ! -f "$log" ]; then
  echo "tests ran: missing log $log" >&2
  exit 1
fi
rust="$(grep -E '[0-9]+ tests run:' "$log" | tail -n 1 | sed -n 's/.* \([0-9][0-9]*\) tests run:.*/\1/p')"
if [ -z "$rust" ]; then
  echo "tests ran: no nextest summary in $log" >&2
  exit 1
fi
pnpm_out="${3:-}"
web=0
if [ -n "$pnpm_out" ] && [ -f "$pnpm_out" ]; then
  web="$(grep -E 'Tests +[0-9]+ passed' "$pnpm_out" | sed -n 's/.*Tests  *\([0-9][0-9]*\) passed.*/\1/p' | awk 'BEGIN { s = 0 } { s += $1 } END { print s + 0 }')"
fi
total=$((rust + web))
echo "$label tests ran: $total"
