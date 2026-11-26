#!/bin/sh
# Pack the Linux desktop binary into the archive install-n24 asks for.
# The .deb and .rpm stay as Tauri built them. This archive is the third format.
set -eu
triple="${1:?target triple}"
case "$triple" in
  *[!A-Za-z0-9_-]*)
    echo "target triple has unexpected characters: $triple" >&2
    exit 1
    ;;
esac
root="$(CDPATH= cd -- "$(dirname "$0")/../.." && pwd)"
bin="$root/target/$triple/release/anybot-desktop"
if [ ! -f "$bin" ]; then
  echo "missing desktop binary at target/$triple/release/anybot-desktop" >&2
  exit 1
fi
stage="$root/target/$triple/release/bundle/archive-stage"
out_dir="$root/target/$triple/release/bundle/archive"
rm -rf "$stage"
mkdir -p "$stage/anybot-desktop" "$out_dir"
cp "$bin" "$stage/anybot-desktop/anybot-desktop"
chmod 755 "$stage/anybot-desktop/anybot-desktop"
archive="$out_dir/anybot-desktop-$triple.tar.gz"
tar -C "$stage" -czf "$archive" anybot-desktop
test -s "$archive"
echo "$archive"
