#!/bin/sh
# Publish the one dev pre-release, or create the stable draft.
# Dev uploads every new file first. Old assets are deleted only after that upload
# succeeds, and the dev-build tag moves only after the page notes match this commit.
# A failed step leaves the previous page and the previous tag untouched.
set -eu
digest="${IMAGE_DIGEST:-}"

prerelease="false"
draft="true"
if [ "$CHANNEL" = "dev" ]; then
  prerelease="true"
  draft="false"
elif printf '%s' "$TAG" | grep -Eq -- '-rc[.][0-9]+$'; then
  prerelease="true"
fi

flags=""
if [ "$draft" = "true" ]; then
  flags="--draft"
fi
if [ "$prerelease" = "true" ]; then
  flags="$flags --prerelease"
fi

page="target/release-page"
notes="target/release-notes.md"
if [ ! -d "$page" ] || [ ! -f "$notes" ]; then
  echo "release page or notes are missing" >&2
  exit 1
fi

if [ "$CHANNEL" = "dev" ]; then
  if [ "$TAG" != "dev-build" ]; then
    echo "dev channel publishes only the dev-build tag" >&2
    exit 1
  fi
  case "$digest" in
    sha256:*) ;;
    *)
      echo "dev publish requires the image digest so the page matches the image" >&2
      exit 1
      ;;
  esac
  if ! grep -q "$SHA" "$notes" || ! grep -q "$digest" "$notes"; then
    echo "notes do not name this commit and image digest" >&2
    exit 1
  fi
  if gh release view "$TAG" >/dev/null 2>&1; then
    old="$(gh release view "$TAG" --json assets --jq '.assets[].name')"
    # shellcheck disable=SC2086
    gh release upload "$TAG" --clobber "$page"/*
    new="$(find "$page" -type f -exec basename {} \;)"
    printf '%s\n' "$old" | while IFS= read -r name; do
      [ -n "$name" ] || continue
      if ! printf '%s\n' "$new" | grep -qx "$name"; then
        gh release delete-asset "$TAG" "$name" --yes
      fi
    done
    gh release edit "$TAG" --notes-file "$notes" --target "$SHA"
  else
    # shellcheck disable=SC2086
    gh release create "$TAG" \
      --title "$TAG" \
      --target "$SHA" \
      --notes-file "$notes" \
      $flags \
      "$page"/*
  fi
  git tag -f "$TAG" "$SHA"
  git push origin "refs/tags/${TAG}" --force
  exit 0
fi

# shellcheck disable=SC2086
gh release create "$TAG" \
  --title "$TAG" \
  --target "$SHA" \
  --notes-file "$notes" \
  $flags \
  "$page"/*
