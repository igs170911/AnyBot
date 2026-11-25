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

trap 'kill -TERM $seed_pid 2>/dev/null || true; wait $seed_pid 2>/dev/null || true' EXIT

for _ in $(seq 1 240); do
  [ -s "$url_file" ] && break
  sleep 0.5
done
[ -s "$url_file" ] || { echo "sqlx seed did not write the url file" >&2; exit 1; }
db_url="$(cat "$url_file")"

if [ "$mode" = "check" ]; then
  SQLX_OFFLINE=false DATABASE_URL="$db_url" "$cargo" sqlx prepare --workspace --all --check -- --all-targets
else
  SQLX_OFFLINE=false DATABASE_URL="$db_url" "$cargo" sqlx prepare --workspace --all -- --all-targets
  "$pnpm" exec biome format --write "$root/.sqlx"
fi

kill -TERM "$seed_pid"
wait "$seed_pid"
trap - EXIT
