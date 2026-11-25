#!/bin/bash
set -euo pipefail

if [ "$#" -eq 0 ]; then
	echo "testlock.sh: no command given" >&2
	exit 2
fi

lock_path="$(git rev-parse --git-common-dir)/anybot-tests.lock"
mkdir -p "$(dirname "$lock_path")"

if command -v flock >/dev/null 2>&1; then
	exec 9>"$lock_path"
	flock -n 9 || echo "waiting for test lock: $lock_path"
	flock -x 9
	exec "$@"
fi

if command -v lockf >/dev/null 2>&1; then
	lockf -t 0 -s -w "$lock_path" true || echo "waiting for test lock: $lock_path"
	exec lockf -k -w "$lock_path" "$@"
fi

exec "$@"
