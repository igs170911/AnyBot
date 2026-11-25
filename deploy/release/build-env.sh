#!/bin/sh
# Print the compile-time release values, one KEY=value per line.
# ANYBOT_GIT_DESCRIBE and SOURCE_DATE_EPOCH are the only values that vary per commit.
# The server image has no .git (see .dockerignore); it must not run this script.
set -eu

channel="${ANYBOT_RELEASE_CHANNEL:-stable}"
case "$channel" in
  stable|dev) ;;
  *)
    echo "ANYBOT_RELEASE_CHANNEL must be stable or dev" >&2
    exit 1
    ;;
esac

if [ -n "${GITHUB_SHA:-}" ]; then
  head_sha="$(git rev-parse HEAD)" || exit 1
  if [ "$head_sha" != "$GITHUB_SHA" ]; then
    echo "checkout HEAD $head_sha is not $GITHUB_SHA" >&2
    exit 1
  fi
fi
# --dirty cannot be combined with a commit-ish. A tagged checkout is still clean.
describe="$(git describe --tags --always --abbrev=12 --dirty)" || exit 1
case "$describe" in
  *-dirty|*-dirty*)
    echo "refusing a dirty worktree ($describe)" >&2
    exit 1
    ;;
esac

if [ -n "${SOURCE_DATE_EPOCH:-}" ]; then
  epoch="$SOURCE_DATE_EPOCH"
elif [ -n "${GITHUB_SHA:-}" ]; then
  epoch="$(git log -1 --format=%ct "$GITHUB_SHA")"
else
  epoch="$(git log -1 --format=%ct HEAD)"
fi

short="$(printf '%s' "${GITHUB_SHA:-$(git rev-parse HEAD)}" | cut -c1-12)"

printf '%s\n' \
  "ANYBOT_RELEASE_CHANNEL=$channel" \
  "ANYBOT_GIT_DESCRIBE=$describe" \
  "SOURCE_DATE_EPOCH=$epoch" \
  "ANYBOT_COMMIT_SHORT=$short"
