#!/bin/sh
# Rename installer files before SHA256SUMS is written.
# GitHub renames release asset names that contain spaces and other non-alphanumeric
# characters (REST: Upload a release asset). A space in SHA256SUMS would not match
# the stored name. Hyphens, underscores, and dots are kept.
set -eu
src="${1:?source directory}"
dest="${2:?destination directory}"
mkdir -p "$dest"
find "$src" -type f ! -name SHA256SUMS -exec sh -c '
  dest="$1"
  shift
  for file do
    base="$(basename "$file")"
    safe="$(printf "%s" "$base" | tr " " "-")"
    case "$safe" in
      *[!A-Za-z0-9._-]*)
        echo "refusing a name GitHub would rename: $base" >&2
        exit 1
        ;;
    esac
    cp "$file" "$dest/$safe"
    echo "$safe"
  done
' sh "$dest" {} +
