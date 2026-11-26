#!/bin/sh
# Fill the release-note template. Writes target/release-notes.md.
set -eu
root="$(CDPATH= cd -- "$(dirname "$0")/../.." && pwd)"
channel="${1:?channel}"
commit="${2:?commit}"
built_at="${3:?built_at}"
tag="${4:?tag}"
digest="${5:-}"
mkdir -p "$root/target"
src="$root/deploy/release/release-notes.md"
out="$root/target/release-notes.md"
python3 - "$src" "$out" "$channel" "$commit" "$built_at" "$tag" "$digest" <<'PY'
import pathlib, sys
src, out, channel, commit, built_at, tag, digest = sys.argv[1:]
text = pathlib.Path(src).read_text(encoding="utf-8")
built = __import__("time").strftime("%Y-%m-%dT%H:%M:%SZ", __import__("time").gmtime(int(built_at)))
for key, value in {
    "{{CHANNEL}}": channel,
    "{{COMMIT}}": commit,
    "{{BUILT_AT}}": built,
    "{{TAG}}": tag,
    "{{IMAGE_DIGEST}}": digest,
}.items():
    text = text.replace(key, value)
pathlib.Path(out).write_text(text.replace("\r\n", "\n").replace("\r", "\n"), encoding="utf-8")
PY
echo "$out"
