#!/bin/sh
# Publish the one dev pre-release, or create the stable draft.
# Dev replaces the fixed page and moves the dev-build tag. It does not push a branch.
set -eu

prerelease="false"
draft="true"
if [ "$CHANNEL" = "dev" ]; then
  prerelease="true"
  draft="false"
elif printf '%s' "$TAG" | grep -Eq -- '-rc[.][0-9]+$'; then
  prerelease="true"
fi

if [ "$CHANNEL" = "dev" ]; then
  if [ "$TAG" != "dev-build" ]; then
    echo "dev channel publishes only the dev-build tag" >&2
    exit 1
  fi
  git tag -f "$TAG" "$SHA"
  git push origin "refs/tags/${TAG}" --force
  gh release delete "$TAG" --yes --cleanup-tag || true
  git tag -f "$TAG" "$SHA"
  git push origin "refs/tags/${TAG}" --force
fi

flags=""
if [ "$draft" = "true" ]; then
  flags="--draft"
fi
if [ "$prerelease" = "true" ]; then
  flags="$flags --prerelease"
fi
# shellcheck disable=SC2086
gh release create "$TAG" \
  --title "$TAG" \
  --notes-file target/release-notes.md \
  $flags \
  target/release-page/*
