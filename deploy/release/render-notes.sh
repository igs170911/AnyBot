#!/bin/sh
# Fill the release-note template. Writes target/release-notes.md.
set -eu
root="$(CDPATH= cd -- "$(dirname "$0")/../.." && pwd)"
channel="${1:?channel}"
commit="${2:?commit}"
built_at="${3:?built_at}"
tag="${4:?tag}"
mkdir -p "$root/target"
src="$root/deploy/release/release-notes.md"
out="$root/target/release-notes.md"
python3 - "$src" "$out" "$channel" "$commit" "$built_at" "$tag" <<'PY'
import pathlib, sys
src, out, channel, commit, built_at, tag = sys.argv[1:]
text = pathlib.Path(src).read_text()
built = __import__("time").strftime("%Y-%m-%dT%H:%M:%SZ", __import__("time").gmtime(int(built_at)))
for key, value in {
    "{{CHANNEL}}": channel,
    "{{COMMIT}}": commit,
    "{{BUILT_AT}}": built,
    "{{TAG}}": tag,
}.items():
    text = text.replace(key, value)
pathlib.Path(out).write_text(text)
PY
echo "$out"
