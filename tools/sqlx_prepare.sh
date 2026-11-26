#!/usr/bin/env bash
# Regenerate (or check) the committed sqlx offline metadata with a throwaway
# harness PostgreSQL. Usage: tools/sqlx_prepare.sh <cargo> <pnpm> check|prepare
set -euo pipefail

cargo="$1"
pnpm="$2"
mode="$3"
root="$(cd "$(dirname "$0")/.." && pwd)"
url_file="$root/target/sqlx_seed.url"
pid_file="$root/target/sqlx_seed.pid"

rm -f "$url_file" "$pid_file"
"$cargo" run -q -p test-harness --bin sqlx_seed -- \
  --url-file "$url_file" --pid-file "$pid_file" &
seed_pid=$!

wait_deadline() {
  local pid="$1"
  local seconds="$2"
  local ticks=$((seconds * 2))
  local tick=0
  while [ "$tick" -lt "$ticks" ]; do
    if ! kill -0 "$pid" 2>/dev/null; then
      wait "$pid" 2>/dev/null || true
      return 0
    fi
    sleep 0.5
    tick=$((tick + 1))
  done
  echo "sqlx seed PID $pid did not exit within ${seconds}s" >&2
  kill -TERM "$pid" 2>/dev/null || true
  sleep 2
  if kill -0 "$pid" 2>/dev/null; then
    kill -KILL "$pid" 2>/dev/null || true
  fi
  wait "$pid" 2>/dev/null || true
  return 1
}

trap 'kill -TERM $seed_pid 2>/dev/null || true; wait_deadline $seed_pid 15 || true' EXIT

for _ in $(seq 1 240); do
  [ -s "$url_file" ] && break
  if ! kill -0 "$seed_pid" 2>/dev/null; then
    echo "sqlx seed exited before writing the url file" >&2
    exit 1
  fi
  sleep 0.5
done
[ -s "$url_file" ] || { echo "sqlx seed did not write the url file within 120s" >&2; exit 1; }
db_url="$(cat "$url_file")"

if [ "$mode" = "check" ]; then
  SQLX_OFFLINE=false DATABASE_URL="$db_url" "$cargo" sqlx prepare --workspace --all --check -- --all-targets
else
  SQLX_OFFLINE=false DATABASE_URL="$db_url" "$cargo" sqlx prepare --workspace --all -- --all-targets
  "$pnpm" exec biome format --write "$root/.sqlx"
fi

kill -TERM "$seed_pid"
wait_deadline "$seed_pid" 15
trap - EXIT
