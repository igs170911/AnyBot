#!/bin/bash
# Install pinned actionlint 1.7.12. Checksums are from the release checksums file.
set -euo pipefail
dest="${1:?dest}"
version="1.7.12"
case "$(uname -s)-$(uname -m)" in
  Darwin-arm64)
    asset="actionlint_${version}_darwin_arm64.tar.gz"
    sha="aba9ced2dee8d27fecca3dc7feb1a7f9a52caefa1eb46f3271ea66b6e0e6953f"
    ;;
  Darwin-x86_64)
    asset="actionlint_${version}_darwin_amd64.tar.gz"
    sha="5b44c3bc2255115c9b69e30efc0fecdf498fdb63c5d58e17084fd5f16324c644"
    ;;
  Linux-x86_64)
    asset="actionlint_${version}_linux_amd64.tar.gz"
    sha="8aca8db96f1b94770f1b0d72b6dddcb1ebb8123cb3712530b08cc387b349a3d8"
    ;;
  Linux-aarch64|Linux-arm64)
    asset="actionlint_${version}_linux_arm64.tar.gz"
    sha="325e971b6ba9bfa504672e29be93c24981eeb1c07576d730e9f7c8805afff0c6"
    ;;
  *)
    echo "install-actionlint: unsupported platform $(uname -s)-$(uname -m)" >&2
    exit 1
    ;;
esac
url="https://github.com/rhysd/actionlint/releases/download/v${version}/${asset}"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
curl -sLf "$url" -o "$tmp/actionlint.tar.gz"
if command -v sha256sum >/dev/null 2>&1; then
  printf '%s  %s\n' "$sha" "$tmp/actionlint.tar.gz" | sha256sum -c -
else
  printf '%s  %s\n' "$sha" "$tmp/actionlint.tar.gz" | shasum -a 256 -c -
fi
tar -xzf "$tmp/actionlint.tar.gz" -C "$tmp" actionlint
mkdir -p "$(dirname "$dest")"
mv "$tmp/actionlint" "$dest"
chmod 755 "$dest"
"$dest" -version
