#!/bin/sh
# Append the compile-time release values to $GITHUB_ENV.
set -eu
root="$(CDPATH= cd -- "$(dirname "$0")/../.." && pwd)"
# shellcheck disable=SC1091
. "$root/deploy/release/build-env.sh" > "$root/target/release-build-env"
while IFS= read -r line; do
  printf '%s\n' "$line" >> "${GITHUB_ENV:?GITHUB_ENV is required}"
  printf '%s\n' "$line"
done < "$root/target/release-build-env"
