#!/bin/sh
# Decisions in deploy/release/*.sh, with fake queries and fake side effects.
# Does not call GitHub or Docker.
set -eu
root="$(CDPATH= cd -- "$(dirname "$0")/../.." && pwd)"
cd "$root"
fail=0

say() {
  printf '%s\n' "$1"
}

need() {
  label="$1"
  got="$2"
  want="$3"
  if [ "$got" = "$want" ]; then
    say "ok $label"
  else
    say "FAIL $label: got [$got] want [$want]"
    fail=1
  fi
}

need_ok() {
  label="$1"
  if [ "$2" -eq 0 ]; then
    say "ok $label"
  else
    say "FAIL $label: expected success, exit $2"
    fail=1
  fi
}

run_guard() {
  out="$1"
  shift
  # shellcheck disable=SC2086
  env GITHUB_OUTPUT="$out" \
    GITHUB_REPOSITORY="${GITHUB_REPOSITORY:-igs170911/AnyBot}" \
    REPOSITORY="${REPOSITORY:-igs170911/AnyBot}" \
    BUILT_AT="${BUILT_AT:-1732523400}" \
    GIT_DESCRIBE="${GIT_DESCRIBE:-v0.1.0-0-gabc}" \
    "$@" sh deploy/release/guard.sh >"$out.stdout" 2>"$out.stderr" || return $?
}

mkdir -p target/release-selftest
base="target/release-selftest"

say "--- candidate tag ---"
rm -f "$base/rc.out"
set +e
run_guard "$base/rc.out" \
  CALL_MODE="" CALL_COMMIT="" EVENT_NAME=push REF_TYPE=tag REF_NAME=v0.1.0-rc.1 \
  SHA=aaaaaaaaaaaa REF_BRANCH=main DEV_HEAD=bbbbbbbbbbbb \
  MAIN_ANCESTOR=yes MAIN_CHECK=success
rc=$?
set -e
need_ok "rc tag is accepted" "$rc"
need "rc image tag" "$(grep '^image_tag=' "$base/rc.out" | cut -d= -f2-)" "0.1.0-rc.1"
need "rc channel" "$(grep '^channel=' "$base/rc.out" | cut -d= -f2-)" "stable"
need "rc describe has a commit" "$(grep '^describe=' "$base/rc.out" | grep -c -- '-g' || true)" "1"

say "--- stable tag ---"
rm -f "$base/stable.out"
set +e
run_guard "$base/stable.out" \
  CALL_MODE="" CALL_COMMIT="" EVENT_NAME=push REF_TYPE=tag REF_NAME=v0.1.0 \
  SHA=aaaaaaaaaaaa REF_BRANCH=main DEV_HEAD=bbbbbbbbbbbb \
  MAIN_ANCESTOR=yes MAIN_CHECK=success
rc=$?
set -e
need_ok "stable tag is accepted" "$rc"
need "stable image tag" "$(grep '^image_tag=' "$base/stable.out" | cut -d= -f2-)" "0.1.0"

say "--- tag not on main ---"
rm -f "$base/off.out"
set +e
run_guard "$base/off.out" \
  CALL_MODE="" CALL_COMMIT="" EVENT_NAME=push REF_TYPE=tag REF_NAME=v0.1.0 \
  SHA=aaaaaaaaaaaa REF_BRANCH=main DEV_HEAD=bbbbbbbbbbbb \
  MAIN_ANCESTOR=no MAIN_CHECK=success
rc=$?
set -e
need "tag not on main" "$rc" "1"

say "--- main check still running ---"
rm -f "$base/running.out"
set +e
run_guard "$base/running.out" \
  CALL_MODE="" CALL_COMMIT="" EVENT_NAME=push REF_TYPE=tag REF_NAME=v0.1.0 \
  SHA=aaaaaaaaaaaa REF_BRANCH=main DEV_HEAD=bbbbbbbbbbbb \
  MAIN_ANCESTOR=yes MAIN_CHECK=running
rc=$?
set -e
need "main check running" "$rc" "1"
need "running reason" "$(grep -c 'not finished' "$base/running.out.stderr" || true)" "1"

say "--- main failed, dev succeeded ---"
rm -f "$base/devgreen.out"
set +e
run_guard "$base/devgreen.out" \
  CALL_MODE="" CALL_COMMIT="" EVENT_NAME=push REF_TYPE=tag REF_NAME=v0.1.0 \
  SHA=aaaaaaaaaaaa REF_BRANCH=main DEV_HEAD=bbbbbbbbbbbb \
  MAIN_ANCESTOR=yes MAIN_CHECK=failure DEV_CHECK=success
rc=$?
set -e
need "dev green does not count" "$rc" "1"

say "--- rerun an old dev workflow ---"
rm -f "$base/olddev.out"
set +e
run_guard "$base/olddev.out" \
  CALL_MODE=dev CALL_COMMIT=aaaaaaaaaaaa EVENT_NAME=push REF_TYPE=branch REF_NAME=dev \
  SHA=aaaaaaaaaaaa REF_BRANCH=dev DEV_HEAD=bbbbbbbbbbbb \
  MAIN_ANCESTOR=yes MAIN_CHECK=success
rc=$?
set -e
need "old dev commit" "$rc" "1"

say "--- current dev tip ---"
rm -f "$base/dev.out"
set +e
run_guard "$base/dev.out" \
  CALL_MODE=dev CALL_COMMIT=bbbbbbbbbbbb EVENT_NAME=push REF_TYPE=branch REF_NAME=dev \
  SHA=bbbbbbbbbbbb REF_BRANCH=dev DEV_HEAD=bbbbbbbbbbbb \
  MAIN_ANCESTOR=yes MAIN_CHECK=success
rc=$?
set -e
need_ok "current dev tip" "$rc"
need "dev image tag" "$(grep '^image_tag=' "$base/dev.out" | cut -d= -f2-)" "dev"

say "--- rehearsal stays rehearsal ---"
rm -f "$base/reh.out"
set +e
run_guard "$base/reh.out" \
  CALL_MODE="" CALL_COMMIT="" EVENT_NAME=workflow_dispatch REF_TYPE=branch REF_NAME=ws/rel-1 \
  SHA=cccccccccccc REF_BRANCH=ws/rel-1 DEV_HEAD=bbbbbbbbbbbb \
  MAIN_ANCESTOR=no MAIN_CHECK=none
rc=$?
set -e
need_ok "rehearsal" "$rc"
need "rehearsal mode" "$(grep '^mode=' "$base/reh.out" | cut -d= -f2-)" "rehearsal"

say "--- vdev does not move latest ---"
set +e
TAG_NAME=vdev REPOSITORY=igs170911/AnyBot LATEST_VERSION=0.1.0 \
  sh deploy/release/move-latest.sh >"$base/vdev.out" 2>"$base/vdev.err"
rc=$?
set -e
need "vdev" "$rc" "1"

say "--- older than latest ---"
set +e
TAG_NAME=v0.1.0 REPOSITORY=igs170911/AnyBot LATEST_VERSION=0.2.0 \
  sh deploy/release/move-latest.sh >"$base/old.out" 2>"$base/old.err"
rc=$?
set -e
need "older than latest" "$rc" "1"

say "--- newer than latest plans a move ---"
set +e
TAG_NAME=v0.2.0 REPOSITORY=igs170911/AnyBot LATEST_VERSION=0.1.0 \
  ANYBOT_DOCKER=echo \
  sh deploy/release/move-latest.sh >"$base/new.out" 2>"$base/new.err"
rc=$?
set -e
need_ok "newer latest" "$rc"
need "latest plan" "$(grep -c 'imagetools create' "$base/new.out" || true)" "1"

say "--- candidate image tag is the rc, not the cargo version ---"
mkdir -p "$base/parts"
printf 'amd64\n' > "$base/parts/amd64.tar"
printf 'arm64\n' > "$base/parts/arm64.tar"
: > "$base/push-out"
set +e
CHANNEL=stable VERSION=0.1.0 IMAGE_TAG=0.1.0-rc.1 COMMIT=aaaaaaaaaaaa \
  IMAGE=ghcr.io/igs170911/anybot IMAGE_DIR="$base/parts" \
  AMD64_DIGEST=sha256:aaa ARM64_DIGEST=sha256:bbb \
  INDEX_DIGEST=sha256:ccc IMAGE_META="$base/index.json" GITHUB_OUTPUT="$root/$base/push-out" \
  ANYBOT_DOCKER=echo \
  sh deploy/release/push-images.sh >"$base/push.out" 2>"$base/push.err"
rc=$?
set -e
need_ok "push rc" "$rc"
need "rc tag planned" "$(grep -c -- '--tag ghcr.io/igs170911/anybot:0.1.0-rc.1' "$base/push.out" || true)" "1"
need "stable tag not planned" "$(grep -c 'ghcr.io/igs170911/anybot:0.1.0 ' "$base/push.out" || true)" "0"
need "digest source" "$(grep -c 'sha256:aaa' "$base/push.out" || true)" "1"
need "index digest emitted" "$(grep '^digest=' "$base/push-out" | cut -d= -f2-)" "sha256:ccc"
need "no shared verify tag" "$(grep -c ':verify' "$base/push.out" || true)" "0"

say "--- a real push uses the digest docker printed, not a shared tag ---"
mkdir -p "$base/bin"
python3 - "$base/bin/docker" <<'PY'
import pathlib, sys
pathlib.Path(sys.argv[1]).write_text(
    "#!/bin/sh\n"
    "printf '%s\\n' \"$*\" >> \"$DOCKER_LOG\"\n"
    "if [ \"$1\" = load ]; then printf '%s\\n' 'Loaded image: ghcr.io/example:local'; exit 0; fi\n"
    "if [ \"$1\" = push ]; then printf '%s\\n' \"$2: digest: sha256:pushed size: 1\"; exit 0; fi\n"
    "if [ \"$1\" = buildx ]; then\n"
    "  meta=\"\"\n"
    "  prev=\"\"\n"
    "  for arg in \"$@\"; do\n"
    "    if [ \"$prev\" = --metadata-file ]; then meta=\"$arg\"; fi\n"
    "    prev=\"$arg\"\n"
    "  done\n"
    "  printf '%s\\n' '{\"containerimage.descriptor\":{\"digest\":\"sha256:index\"}}' > \"$meta\"\n"
    "  exit 0\n"
    "fi\n"
    "exit 0\n",
    encoding="utf-8",
)
PY
chmod +x "$base/bin/docker"
: > "$base/docker.log"
: > "$base/push-real-out"
set +e
IMAGE_TAG=0.1.0 COMMIT=aaaaaaaaaaaa IMAGE=ghcr.io/igs170911/anybot \
  IMAGE_DIR="$base/parts" IMAGE_META="$base/real-index.json" \
  GITHUB_OUTPUT="$root/$base/push-real-out" DOCKER_LOG="$root/$base/docker.log" \
  ANYBOT_DOCKER="$root/$base/bin/docker" \
  sh deploy/release/push-images.sh >"$base/push-real.out" 2>"$base/push-real.err"
rc=$?
set -e
need_ok "real push" "$rc"
need "real push has no verify tag" "$(grep -c ':verify' "$base/docker.log" || true)" "0"
need "index built from pushed digests" "$(grep -c '@sha256:pushed' "$base/docker.log" || true)" "1"
need "real index digest" "$(grep '^digest=' "$base/push-real-out" | cut -d= -f2-)" "sha256:index"

say "--- spaces are removed before the checksum ---"
src="$base/spaced"
out="$base/page"
rm -rf "$src" "$out"
mkdir -p "$src"
printf 'exe\n' > "$src/AnyBot Dev_0.1.0_x64-setup.exe"
sh deploy/release/sanitize-names.sh "$src" "$out" >"$base/names.out"
need "space removed" "$(find "$out" -name '* *' | wc -l | tr -d ' ')" "0"
need "hyphenated name" "$(find "$out" -name 'AnyBot-Dev_0.1.0_x64-setup.exe' | wc -l | tr -d ' ')" "1"

say "--- dev upload failure leaves the old page ---"
work="$root/$base/work"
rm -rf "$work"
mkdir -p "$work/target/release-page" "$work/bin"
printf 'new\n' > "$work/target/release-page/new.txt"
printf 'commit bbbbbbbbbbbb digest sha256:ccc\n' > "$work/target/release-notes.md"
python3 - "$work/bin/gh" "$work/bin/git" <<'PY'
import pathlib, sys
gh, git = sys.argv[1:]
pathlib.Path(gh).write_text(
    "#!/bin/sh\n"
    "printf '%s\\n' \"$*\" >> \"$GH_LOG\"\n"
    "case \"$1 $2\" in\n"
    "  \"release view\") exit 0 ;;\n"
    "  \"release upload\") exit 1 ;;\n"
    "esac\n"
    "exit 0\n",
    encoding="utf-8",
)
pathlib.Path(git).write_text(
    "#!/bin/sh\nprintf '%s\\n' \"$*\" >> \"$GIT_LOG\"\nexit 0\n",
    encoding="utf-8",
)
PY
chmod +x "$work/bin/gh" "$work/bin/git"
: > "$base/gh-fail.log"
: > "$base/git-fail.log"
set +e
here="$(pwd)"
cd "$work"
CHANNEL=dev TAG=dev-build SHA=bbbbbbbbbbbb IMAGE_DIGEST=sha256:ccc \
  GH_LOG="$root/$base/gh-fail.log" GIT_LOG="$root/$base/git-fail.log" \
  PATH="$work/bin:/usr/bin:/bin" \
  sh "$root/deploy/release/publish-release.sh" >"$root/$base/pub.out" 2>"$root/$base/pub.err"
rc=$?
cd "$here"
set -e
need "upload failure refuses" "$rc" "1"
need "tag not moved after a failed upload" "$(grep -c 'tag -f' "$base/git-fail.log" || true)" "0"
need "page not edited after a failed upload" "$(grep -c 'release edit' "$base/gh-fail.log" || true)" "0"

say "--- dev upload then tag move ---"
python3 - "$work/bin/gh" <<'PY'
import pathlib, sys
pathlib.Path(sys.argv[1]).write_text(
    "#!/bin/sh\n"
    "printf '%s\\n' \"$*\" >> \"$GH_LOG\"\n"
    "case \"$1 $2\" in\n"
    "  \"release view\")\n"
    "    if [ \"${GH_ASSETS:-}\" = 1 ]; then printf '%s\\n' old.txt; fi\n"
    "    exit 0 ;;\n"
    "esac\n"
    "exit 0\n",
    encoding="utf-8",
)
PY
chmod +x "$work/bin/gh"
: > "$base/gh.log"
: > "$base/git.log"
set +e
cd "$work"
CHANNEL=dev TAG=dev-build SHA=bbbbbbbbbbbb IMAGE_DIGEST=sha256:ccc \
  GH_LOG="$root/$base/gh.log" GIT_LOG="$root/$base/git.log" GH_ASSETS=1 \
  PATH="$work/bin:/usr/bin:/bin" \
  sh "$root/deploy/release/publish-release.sh" >"$root/$base/pub2.out" 2>"$root/$base/pub2.err"
rc=$?
cd "$here"
set -e
need "dev publish succeeds" "$rc" "0"
need "old asset deleted after upload" "$(grep -c 'delete-asset dev-build old.txt' "$base/gh.log" || true)" "1"
need "tag moved after the page update" "$(grep -c 'tag -f' "$base/git.log" || true)" "1"

if [ "$fail" -ne 0 ]; then
  say "release-selftest: failed"
  exit 1
fi
say "release-selftest: pass"
