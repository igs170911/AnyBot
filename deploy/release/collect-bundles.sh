#!/bin/sh
# Copy desktop installers for one Rust target into dest and write SHA256SUMS.
# Tauri writes to the repo-root target/<triple>/release/bundle, not under src-tauri.
# Deb intermediates (control.tar.gz, data.tar.gz) are not downloads.
# Usage: collect-bundles.sh <dest> <target-triple>
set -eu
dest="${1:?dest}"
triple="${2:?target triple}"
case "$triple" in
  *[!A-Za-z0-9_-]*)
    echo "target triple has unexpected characters: $triple" >&2
    exit 1
    ;;
esac
root="$(CDPATH= cd -- "$(dirname "$0")/../.." && pwd)"
bundle="$root/target/$triple/release/bundle"
if [ ! -d "$bundle" ]; then
  echo "no bundle directory at target/$triple/release/bundle" >&2
  exit 1
fi
mkdir -p "$dest"
find "$bundle" -type f -exec sh -c '
  dest="$1"
  shift
  keep() {
    case "$(basename "$1")" in
      control.tar.gz|data.tar.gz) return 1 ;;
    esac
    case "$1" in
      *.dmg|*.app.tar.gz|*.exe|*.msi|*.deb|*.rpm|*.AppImage) return 0 ;;
      */archive/*.tar.gz) return 0 ;;
      *) return 1 ;;
    esac
  }
  for file do
    if keep "$file"; then
      cp "$file" "$dest/$(basename "$file")"
    fi
  done
' sh "$dest" {} +
count="$(find "$dest" -type f ! -name SHA256SUMS | wc -l | tr -d ' ')"
if [ "$count" = 0 ]; then
  echo "bundle directory exists but no installers were found in target/$triple/release/bundle" >&2
  exit 1
fi
(
  cd "$dest"
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum -- * > SHA256SUMS
  else
    shasum -a 256 -- * > SHA256SUMS
  fi
  grep -v ' SHA256SUMS$' SHA256SUMS > SHA256SUMS.tmp || true
  mv SHA256SUMS.tmp SHA256SUMS
)
echo "collected $count installer(s) into $dest"
