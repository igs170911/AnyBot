#!/usr/bin/env bash
set -euo pipefail

cargo="${1:-cargo}"
package="${2:-}"
filter="${3:-}"
root="$(cd "$(dirname "$0")/.." && pwd)"

if [ -z "$package" ]; then
	echo "usage: make test-pkg P=<package> [T=<test name fragment>]" >&2
	exit 2
fi

mkdir -p "$root/target"
args=(nextest run --profile quick -p "$package")
if [ -n "$filter" ]; then
	args+=("$filter")
fi
exec bash "$root/tools/testlock.sh" bash "$root/tools/test-gate.sh" run "$root/target/test-pkg.log" "$cargo" "${args[@]}"
