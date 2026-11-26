#!/bin/sh
# Publish the one dev pre-release, or create the stable draft.
# Dev never edits the visible page in place. A complete new draft is uploaded
# under a throwaway tag, then the visible page is deleted and the draft is
# published onto dev-build. A failed step deletes only the throwaway draft.
# The dev-build git tag moves only after that swap. The caller moves :dev only
# after this script exits 0.
# https://cli.github.com/manual/gh_release_upload (--clobber deletes first)
# https://cli.github.com/manual/gh_release_create (draft, then publish)
# https://docs.github.com/en/rest/releases/assets (same name is HTTP 422)
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
  staging="dev-build-next-${SHA}"
  # A leftover draft from an earlier failed attempt is not the visible page.
  gh release delete "$staging" --yes --cleanup-tag >/dev/null 2>&1 || true
  # shellcheck disable=SC2086
  if ! gh release create "$staging" \
    --draft \
    --title "$TAG" \
    --target "$SHA" \
    --notes-file "$notes" \
    "$page"/*; then
    gh release delete "$staging" --yes --cleanup-tag >/dev/null 2>&1 || true
    echo "dev draft was not complete; the visible page is unchanged" >&2
    exit 1
  fi
  # Publish the complete draft under its own tag first. The visible dev-build
  # page is still the old one. Only then delete the old page and retag.
  # gh release edit --tag renames the release; it does not copy assets.
  if ! gh release edit "$staging" --draft=false --prerelease --latest=false \
    --target "$SHA" --notes-file "$notes"; then
    gh release delete "$staging" --yes --cleanup-tag >/dev/null 2>&1 || true
    echo "dev draft was not published; the visible page is unchanged" >&2
    exit 1
  fi
  if gh release view "$TAG" >/dev/null 2>&1; then
    gh release delete "$TAG" --yes
  fi
  if ! gh release edit "$staging" --tag "$TAG" --target "$SHA" --notes-file "$notes"; then
    echo "dev page rename failed; $staging is the complete new page and $TAG may be gone" >&2
    exit 1
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
