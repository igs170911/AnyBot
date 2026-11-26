#!/bin/sh
# Publish the one dev pre-release, or create the stable draft.
# Dev never edits the visible page in place. A complete draft is uploaded under
# a throwaway name. The old Release and its git tag are deleted together, then
# one edit renames the draft onto dev-build and publishes it. GitHub creates
# the dev-build tag in that edit, so this script does not push a tag.
# A failed upload deletes only the throwaway draft. The caller moves :dev only
# after this script exits 0.
# https://cli.github.com/manual/gh_release_delete (--cleanup-tag deletes the tag)
# https://cli.github.com/manual/gh_release_edit (--tag renames; --draft=false publishes)
# https://docs.github.com/en/rest/releases/releases (one PATCH sets tag_name and draft)
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
  # --cleanup-tag removes its git tag too, if one was created.
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
  # Delete the old Release and its tag together, then rename and publish in
  # one edit. Publishing first under the throwaway name would leave that tag
  # behind: edit --tag creates the new tag and does not delete the old one.
  # https://cli.github.com/manual/gh_release_delete
  # https://docs.github.com/en/rest/releases/releases
  if gh release view "$TAG" >/dev/null 2>&1; then
    gh release delete "$TAG" --yes --cleanup-tag
  fi
  if ! gh release edit "$staging" \
    --tag "$TAG" \
    --draft=false \
    --prerelease \
    --latest=false \
    --target "$SHA" \
    --notes-file "$notes"; then
    gh release delete "$staging" --yes --cleanup-tag >/dev/null 2>&1 || true
    echo "dev page was not published; dev-build was already removed" >&2
    exit 1
  fi
  exit 0
fi

# shellcheck disable=SC2086
gh release create "$TAG" \
  --title "$TAG" \
  --target "$SHA" \
  --notes-file "$notes" \
  $flags \
  "$page"/*
