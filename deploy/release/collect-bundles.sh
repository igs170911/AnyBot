#!/bin/sh
# Copy desktop installers into target/release-assets and write SHA256SUMS.
set -eu
dest="${1:-target/release-assets}"
mkdir -p "$dest"
for dir in apps/desktop/src-tauri/target/*/release/bundle apps/desktop/src-tauri/target/release/bundle; do
  [ -d "$dir" ] || continue
  find "$dir" -type f \( \
    -name '*.dmg' -o -name '*.app.tar.gz' -o -name '*.exe' -o -name '*.msi' -o \
    -name '*.deb' -o -name '*.rpm' -o -name '*.AppImage' \
    \) -print | while IFS= read -r file; do
    cp "$file" "$dest/$(basename "$file")"
  done
done
count="$(find "$dest" -type f ! -name SHA256SUMS | wc -l | tr -d ' ')"
if [ "$count" = 0 ]; then
  echo "no installers found under apps/desktop/src-tauri/target" >&2
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
