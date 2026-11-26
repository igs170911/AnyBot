#!/bin/sh
# Append the compile-time release values to $GITHUB_ENV.
# A fresh checkout has no target/. Create it before writing the copy.
set -eu
root="$(CDPATH= cd -- "$(dirname "$0")/../.." && pwd)"
mkdir -p "$root/target"
# shellcheck disable=SC1091
. "$root/deploy/release/build-env.sh" > "$root/target/release-build-env"
while IFS= read -r line || [ -n "$line" ]; do
  [ -n "$line" ] || continue
  printf '%s\n' "$line" >> "${GITHUB_ENV:?GITHUB_ENV is required}"
  printf '%s\n' "$line"
done < "$root/target/release-build-env"
