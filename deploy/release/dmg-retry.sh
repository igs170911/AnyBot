#!/bin/sh
# Build the desktop bundles. On macOS, hdiutil sometimes reports that the disk
# image is still held by another process. That one error is retried.
# Every other error fails immediately.
# create-dmg retries the same message (github.com/create-dmg/create-dmg/pull/198).
# Tauri's vendored bundle_dmg.sh does not. Spotlight is disabled on this
# runner before the build (man mdutil); that does not remove the race.
set -eu
if [ "${DMG_RETRY_SELFTEST:-}" = 1 ]; then
  log="${1:?log}"
  rc="${2:?rc}"
  shift 2
  if [ "$rc" -eq 0 ]; then
    exit 0
  fi
  if ! grep -q 'Resource busy' "$log"; then
    echo "bundle failed with exit $rc; not Resource busy, not retrying" >&2
    exit "$rc"
  fi
  echo "hdiutil Resource busy, retry 1 of 3, waiting 5s"
  exit 75
fi
attempts=3
wait_seconds=5
try=1
while [ "$try" -le "$attempts" ]; do
  log="target/dmg-attempt-${try}.log"
  mkdir -p target
  : > "$log"
  set +e
  "$@" >"$log" 2>&1
  rc=$?
  set -e
  cat "$log"
  if [ "$rc" -eq 0 ]; then
    exit 0
  fi
  if ! grep -q 'Resource busy' "$log"; then
    echo "bundle failed with exit $rc; not Resource busy, not retrying" >&2
    exit "$rc"
  fi
  if [ "$try" -eq "$attempts" ]; then
    echo "bundle still Resource busy after $attempts attempts" >&2
    exit "$rc"
  fi
  echo "hdiutil Resource busy, retry $try of $attempts, waiting ${wait_seconds}s"
  sleep "$wait_seconds"
  try=$((try + 1))
done
exit 1
