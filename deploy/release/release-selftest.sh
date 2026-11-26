#!/bin/sh
# Decisions in deploy/release/*.sh. docker, gh, and git on PATH are stubs.
# The stubs are the only commands those names can resolve to.
# ANYBOT_DOCKER must not point past the stub. Does not call GitHub, Docker, or git.
set -eu
unset ANYBOT_DOCKER || true
root="$(CDPATH= cd -- "$(dirname "$0")/../.." && pwd)"
cd "$root"
fail=0
base="${ANYBOT_SELFTEST_BASE:-target/release-selftest}"
stub="$root/$base/stub"

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

# docker, gh, and git resolve only to the stubs. Other tools stay on PATH.
only_stubs() {
  PATH="$stub:$PATH"
  export PATH
  hash -r 2>/dev/null || true
  for name in docker gh git; do
    found="$(command -v "$name")"
    case "$found" in
      "$stub/$name") ;;
      *)
        echo "stub $name is not the only one on PATH ($found)" >&2
        exit 1
        ;;
    esac
  done
  if [ -n "${ANYBOT_DOCKER:-}" ]; then
    echo "ANYBOT_DOCKER is set ($ANYBOT_DOCKER); refusing to run" >&2
    exit 1
  fi
}

prepare() {
  name="$1"
  rm -rf "$base/$name"
  mkdir -p "$base/$name" "$stub"
  : > "$base/$name/docker.log"
  : > "$base/$name/gh.log"
  : > "$base/$name/git.log"
  export DOCKER_LOG="$root/$base/$name/docker.log"
  export GH_LOG="$root/$base/$name/gh.log"
  export GIT_LOG="$root/$base/$name/git.log"
  export STUB_MODE="${2:-ok}"
  unset LATEST_VERSION || true
  only_stubs
}

mutating() {
  log="$1"
  if [ ! -s "$log" ]; then
    printf '0'
    return
  fi
  grep -Ec '^(docker push|docker tag|docker buildx imagetools create|gh release|git tag|git push)' "$log" || true
}

mkdir -p "$stub"
python3 - "$stub/docker" "$stub/gh" "$stub/git" <<'PY'
import pathlib, sys
docker, gh, git = sys.argv[1:]
pathlib.Path(docker).write_text(
    "#!/bin/sh\n"
    "printf '%s\\n' \"docker $*\" >> \"$DOCKER_LOG\"\n"
    "case \"$STUB_MODE\" in\n"
    "  missing)\n"
    "    if [ \"$1\" = buildx ] && [ \"$3\" = inspect ]; then\n"
    "      echo 'manifest unknown' >&2\n"
    "      if [ -n \"${LATEST_ERR:-}\" ]; then echo 'manifest unknown' > \"$LATEST_ERR\"; fi\n"
    "      exit 1\n"
    "    fi ;;\n"
    "  net)\n"
    "    echo 'dial tcp: connection refused' >&2\n"
    "    exit 1 ;;\n"
    "esac\n"
    "if [ \"$1\" = load ]; then printf '%s\\n' 'Loaded image: ghcr.io/example:local'; exit 0; fi\n"
    "if [ \"$1\" = push ]; then printf '%s\\n' \"$2: digest: sha256:pushed size: 1\"; exit 0; fi\n"
    "if [ \"$1\" = buildx ]; then\n"
    "  meta=\"\"\n"
    "  prev=\"\"\n"
    "  for arg in \"$@\"; do\n"
    "    if [ \"$prev\" = --metadata-file ]; then meta=\"$arg\"; fi\n"
    "    prev=\"$arg\"\n"
    "  done\n"
    "  if [ -n \"$meta\" ]; then\n"
    "    printf '%s\\n' '{\"containerimage.descriptor\":{\"digest\":\"sha256:index\"}}' > \"$meta\"\n"
    "  fi\n"
    "  if [ \"$3\" = inspect ]; then\n"
    "    printf '%s\\n' '0.1.0'\n"
    "  fi\n"
    "  exit 0\n"
    "fi\n"
    "exit 0\n",
    encoding="utf-8",
)
pathlib.Path(gh).write_text(
    "#!/bin/sh\n"
    "printf '%s\\n' \"gh $*\" >> \"$GH_LOG\"\n"
    "case \"$STUB_MODE\" in\n"
    "  upload-fail)\n"
    "    case \"$1 $2\" in\n"
    "      \"release create\") exit 1 ;;\n"
    "    esac ;;\n"
    "  swap-fail)\n"
    "    case \"$*\" in\n"
    "      *--draft=false*) exit 1 ;;\n"
    "    esac ;;\n"
    "esac\n"
    "case \"$1\" in\n"
    "  api)\n"
    "    case \"$*\" in\n"
    "      *workflows/ci.yml*)\n"
    "        case \"${GH_RUN:-9}\" in\n"
    "          none) printf '%s\\n' ''; exit 0 ;;\n"
    "          *) printf '%s\\n' \"${GH_RUN:-9}\"; exit 0 ;;\n"
    "        esac ;;\n"
    "      */jobs*) printf '%s\\n' \"${GH_JOBS:-0}\"; exit 0 ;;\n"
    "    esac\n"
    "    exit 0 ;;\n"
    "esac\n"
    "case \"$1 $2\" in\n"
    "  \"release view\")\n"
    "    case \"$STUB_MODE\" in\n"
    "      no-page) exit 1 ;;\n"
    "    esac\n"
    "    if [ \"${GH_ASSETS:-}\" = 1 ]; then printf '%s\\n' old.txt; fi\n"
    "    exit 0 ;;\n"
    "esac\n"
    "exit 0\n",
    encoding="utf-8",
)
pathlib.Path(git).write_text(
    "#!/bin/sh\n"
    "printf '%s\\n' \"git $*\" >> \"$GIT_LOG\"\n"
    "case \"$1\" in\n"
    "  rev-parse) printf '%s\\n' \"${GIT_TIP:-bbbbbbbbbbbb}\"; exit 0 ;;\n"
    "  merge-base)\n"
    "    if [ \"${GIT_ON_MAIN:-yes}\" = yes ]; then exit 0; fi\n"
    "    exit 1 ;;\n"
    "  describe) printf '%s\\n' \"${GIT_DESCRIBE_OUT:-v0.1.0}\"; exit 0 ;;\n"
    "  log) printf '%s\\n' 1732523400; exit 0 ;;\n"
    "esac\n"
    "exit 0\n",
    encoding="utf-8",
)
for path in (docker, gh, git):
    pathlib.Path(path).chmod(0o755)
PY

run_guard() {
  out="$1"
  shift
  rm -f "$out"
  # shellcheck disable=SC2086
  env GITHUB_OUTPUT="$out" \
    GITHUB_REPOSITORY="${GITHUB_REPOSITORY:-igs170911/AnyBot}" \
    REPOSITORY="${REPOSITORY:-igs170911/AnyBot}" \
    BUILT_AT="${BUILT_AT:-1732523400}" \
    GIT_DESCRIBE="${GIT_DESCRIBE:-}" \
    PATH="$stub:$PATH" \
    "$@" sh deploy/release/guard.sh >"$out.stdout" 2>"$out.stderr" || return $?
}

if [ -n "${ANYBOT_SELFTEST_INNER:-}" ]; then
  skip_to_images=1
else
  skip_to_images=0
fi

if [ "$skip_to_images" -eq 0 ]; then
say "--- candidate tag ---"
prepare rc
set +e
run_guard "$base/rc.out" \
  CALL_MODE="" CALL_COMMIT="" EVENT_NAME=push REF_TYPE=tag REF_NAME=v0.1.0-rc.1 \
  SHA=aaaaaaaaaaaa GH_RUN=9 GH_JOBS=3 GIT_ON_MAIN=yes
rc=$?
set -e
need_ok "rc tag is accepted" "$rc"
need "rc image tag" "$(grep '^image_tag=' "$base/rc.out" | cut -d= -f2-)" "0.1.0-rc.1"
need "rc channel" "$(grep '^channel=' "$base/rc.out" | cut -d= -f2-)" "stable"
need "rc describe has a commit" "$(grep '^describe=' "$base/rc.out" | grep -c -- '-g' || true)" "1"
need "rc asked ci.yml" "$(grep -c 'workflows/ci.yml' "$base/rc/gh.log" || true)" "1"

say "--- stable tag ---"
prepare stable
set +e
run_guard "$base/stable.out" \
  CALL_MODE="" CALL_COMMIT="" EVENT_NAME=push REF_TYPE=tag REF_NAME=v0.1.0 \
  SHA=aaaaaaaaaaaa GH_RUN=9 GH_JOBS=3 GIT_ON_MAIN=yes
rc=$?
set -e
need_ok "stable tag is accepted" "$rc"
need "stable image tag" "$(grep '^image_tag=' "$base/stable.out" | cut -d= -f2-)" "0.1.0"
need "stable asked ci.yml" "$(grep -c 'workflows/ci.yml' "$base/stable/gh.log" || true)" "1"

say "--- tag not on main ---"
prepare off
set +e
run_guard "$base/off.out" \
  CALL_MODE="" CALL_COMMIT="" EVENT_NAME=push REF_TYPE=tag REF_NAME=v0.1.0 \
  SHA=aaaaaaaaaaaa GH_RUN=9 GH_JOBS=3 GIT_ON_MAIN=no
rc=$?
set -e
need "tag not on main" "$rc" "1"
need "off main reason" "$(grep -c 'not on main' "$base/off.out.stderr" || true)" "1"
need "off main did not query checks" "$(grep -c 'workflows/ci.yml' "$base/off/gh.log" || true)" "0"
need "off main no mutating gh" "$(mutating "$base/off/gh.log")" "0"

say "--- main check still running ---"
prepare running
set +e
run_guard "$base/running.out" \
  CALL_MODE="" CALL_COMMIT="" EVENT_NAME=push REF_TYPE=tag REF_NAME=v0.1.0 \
  SHA=aaaaaaaaaaaa GH_RUN=9 GH_JOBS=1 GIT_ON_MAIN=yes
rc=$?
set -e
need "main check running" "$rc" "1"
need "running reason" "$(grep -c 'not finished' "$base/running.out.stderr" || true)" "1"
need "running no mutating" "$(mutating "$base/running/gh.log")" "0"

say "--- main failed, dev succeeded ---"
prepare devgreen
set +e
run_guard "$base/devgreen.out" \
  CALL_MODE="" CALL_COMMIT="" EVENT_NAME=push REF_TYPE=tag REF_NAME=v0.1.0 \
  SHA=aaaaaaaaaaaa GH_RUN=9 GH_JOBS=0 GIT_ON_MAIN=yes
rc=$?
set -e
need "dev green does not count" "$rc" "1"
need "dev green reason" "$(grep -c 'not success' "$base/devgreen.out.stderr" || true)" "1"
need "dev green queried ci" "$(grep -c 'workflows/ci.yml' "$base/devgreen/gh.log" || true)" "1"

say "--- wiki suite is not the ci run ---"
prepare wiki
set +e
run_guard "$base/wiki.out" \
  CALL_MODE="" CALL_COMMIT="" EVENT_NAME=push REF_TYPE=tag REF_NAME=v0.1.0 \
  SHA=aaaaaaaaaaaa GH_RUN=none GIT_ON_MAIN=yes
rc=$?
set -e
need "missing ci run" "$rc" "1"
need "missing ci reason" "$(grep -c 'not success' "$base/wiki.out.stderr" || true)" "1"

say "--- rerun an old dev workflow ---"
prepare olddev
set +e
run_guard "$base/olddev.out" \
  CALL_MODE=dev CALL_COMMIT=aaaaaaaaaaaa EVENT_NAME=push REF_TYPE=branch REF_NAME=dev \
  SHA=aaaaaaaaaaaa GIT_TIP=bbbbbbbbbbbb
rc=$?
set -e
need "old dev commit" "$rc" "1"
need "old dev reason" "$(grep -c 'current tip' "$base/olddev.out.stderr" || true)" "1"
need "old dev used git rev-parse" "$(grep -c 'rev-parse' "$base/olddev/git.log" || true)" "1"

say "--- current dev tip ---"
prepare devtip
set +e
run_guard "$base/dev.out" \
  CALL_MODE=dev CALL_COMMIT=bbbbbbbbbbbb EVENT_NAME=push REF_TYPE=branch REF_NAME=dev \
  SHA=bbbbbbbbbbbb GIT_TIP=bbbbbbbbbbbb
rc=$?
set -e
need_ok "current dev tip" "$rc"
need "dev image tag" "$(grep '^image_tag=' "$base/dev.out" | cut -d= -f2-)" "dev"

say "--- publish job re-checks the tip before a write ---"
prepare recheck stale
set +e
CHANNEL=dev SHA=aaaaaaaaaaaa GIT_TIP=bbbbbbbbbbbb \
  sh deploy/release/recheck-dev.sh >"$base/recheck.out" 2>"$base/recheck.err"
rc=$?
set -e
need "stale queued dev" "$rc" "1"
need "stale queued reason" "$(grep -c 'no longer the tip' "$base/recheck.err" || true)" "1"

say "--- rehearsal stays rehearsal ---"
prepare reh
set +e
run_guard "$base/reh.out" \
  CALL_MODE="" CALL_COMMIT="" EVENT_NAME=workflow_dispatch REF_TYPE=branch REF_NAME=ws/rel-1 \
  SHA=cccccccccccc
rc=$?
set -e
need_ok "rehearsal" "$rc"
need "rehearsal mode" "$(grep '^mode=' "$base/reh.out" | cut -d= -f2-)" "rehearsal"
need "rehearsal did not call gh" "$(wc -c < "$base/reh/gh.log" | tr -d ' ')" "0"

say "--- vdev does not move latest ---"
prepare vdev
set +e
TAG_NAME=vdev REPOSITORY=igs170911/AnyBot LATEST_VERSION=0.1.0 \
  sh deploy/release/move-latest.sh >"$base/vdev.out" 2>"$base/vdev.err"
rc=$?
set -e
need "vdev" "$rc" "1"
need "vdev reason" "$(grep -c 'not v<number>' "$base/vdev.err" || true)" "1"
need "vdev did not call docker" "$(wc -c < "$base/vdev/docker.log" | tr -d ' ')" "0"

say "--- four-part version does not move latest ---"
prepare four
set +e
TAG_NAME=v1.2.3.4 REPOSITORY=igs170911/AnyBot \
  sh deploy/release/move-latest.sh >"$base/four.out" 2>"$base/four.err"
rc=$?
set -e
need "four parts" "$rc" "1"
need "four parts reason" "$(grep -c 'three numeric parts' "$base/four.err" || true)" "1"
need "four parts did not call docker" "$(wc -c < "$base/four/docker.log" | tr -d ' ')" "0"

say "--- leading zero does not move latest ---"
prepare zero
set +e
TAG_NAME=v01.2.3 REPOSITORY=igs170911/AnyBot \
  sh deploy/release/move-latest.sh >"$base/zero.out" 2>"$base/zero.err"
rc=$?
set -e
need "leading zero" "$rc" "1"
need "leading zero reason" "$(grep -c 'leading zeros' "$base/zero.err" || true)" "1"
need "leading zero did not call docker" "$(wc -c < "$base/zero/docker.log" | tr -d ' ')" "0"

say "--- older than latest ---"
prepare older
set +e
TAG_NAME=v0.1.0 REPOSITORY=igs170911/AnyBot LATEST_VERSION=0.2.0 \
  sh deploy/release/move-latest.sh >"$base/old.out" 2>"$base/old.err"
rc=$?
set -e
need "older than latest" "$rc" "1"
need "older reason" "$(grep -c 'refusing to move latest' "$base/old.err" || true)" "1"
need "older did not create" "$(grep -c 'imagetools create' "$base/older/docker.log" || true)" "0"

say "--- unreadable latest does not move ---"
prepare unread net
set +e
rc=0
TAG_NAME=v0.2.0 REPOSITORY=igs170911/AnyBot \
  LATEST_ERR="$base/unread/inspect.err" \
  sh deploy/release/move-latest.sh >"$base/unread.out" 2>"$base/unread.err" || rc=$?
set -e
need "unreadable latest" "$rc" "1"
need "unreadable reason" "$(grep -c 'cannot read' "$base/unread.err" || true)" "1"
need "unreadable did not create" "$(grep -c 'imagetools create' "$base/unread/docker.log" || true)" "0"

say "--- missing latest is a first publish ---"
prepare missing missing
set +e
rc=0
TAG_NAME=v0.2.0 REPOSITORY=igs170911/AnyBot \
  LATEST_ERR="$root/$base/missing/inspect.err" \
  sh deploy/release/move-latest.sh >"$base/missing.out" 2>"$base/missing.err" || rc=$?
set -e
need_ok "missing latest" "$rc"
need "missing planned a move" "$(grep -c 'imagetools create' "$base/missing/docker.log" || true)" "1"
need "missing create has no format flag" "$(grep 'imagetools create' "$base/missing/docker.log" | grep -c -- '--format ' || true)" "0"

say "--- newer than latest plans a move ---"
prepare newer
set +e
TAG_NAME=v0.2.0 REPOSITORY=igs170911/AnyBot LATEST_VERSION=0.1.0 \
  sh deploy/release/move-latest.sh >"$base/new.out" 2>"$base/new.err"
rc=$?
set -e
need_ok "newer latest" "$rc"
need "latest plan" "$(grep -c 'imagetools create' "$base/newer/docker.log" || true)" "1"

fi

say "--- candidate image tag is the rc, not the cargo version ---"
prepare push-rc
mkdir -p "$base/parts"
printf 'amd64\n' > "$base/parts/amd64.tar"
printf 'arm64\n' > "$base/parts/arm64.tar"
: > "$base/push-out"
set +e
CHANNEL=stable VERSION=0.1.0 IMAGE_TAG=0.1.0-rc.1 COMMIT=aaaaaaaaaaaa \
  IMAGE=ghcr.io/igs170911/anybot IMAGE_DIR="$base/parts" \
  AMD64_DIGEST=sha256:aaa ARM64_DIGEST=sha256:bbb \
  INDEX_DIGEST=sha256:ccc IMAGE_META="$base/index.json" GITHUB_OUTPUT="$root/$base/push-out" \
  sh deploy/release/push-images.sh >"$base/push.out" 2>"$base/push.err"
rc=$?
set -e
need_ok "push rc" "$rc"
need "rc commit tag planned" "$(grep -c -- '--tag ghcr.io/igs170911/anybot:aaaaaaaaaaaa' "$base/push-rc/docker.log" || true)" "1"
need "rc version tag planned" "$(grep -c -- '--tag ghcr.io/igs170911/anybot:0.1.0-rc.1' "$base/push-rc/docker.log" || true)" "1"
need "rc did not plan the cargo version" "$(grep -c 'anybot:0.1.0 ' "$base/push-rc/docker.log" || true)" "0"
need "digest source" "$(grep -c 'sha256:aaa' "$base/push-rc/docker.log" || true)" "1"
need "index digest emitted" "$(grep '^digest=' "$base/push-out" | cut -d= -f2-)" "sha256:index"
need "no shared verify tag" "$(grep -c ':verify' "$base/push-rc/docker.log" || true)" "0"
need "create has no format flag" "$(grep 'imagetools create' "$base/push-rc/docker.log" | grep -c -- '--format ' || true)" "0"

say "--- dev image names are not the stable names ---"
prepare push-dev
: > "$base/push-dev-out"
set +e
CHANNEL=dev IMAGE_TAG=dev COMMIT=bbbbbbbbbbbb \
  IMAGE=ghcr.io/igs170911/anybot IMAGE_DIR="$base/parts" \
  AMD64_DIGEST=sha256:aaa ARM64_DIGEST=sha256:bbb \
  INDEX_DIGEST=sha256:ddd IMAGE_META="$base/dev-index.json" GITHUB_OUTPUT="$root/$base/push-dev-out" \
  sh deploy/release/push-images.sh >"$base/push-dev.out" 2>"$base/push-dev.err"
rc=$?
set -e
need_ok "push dev index" "$rc"
need "dev arch tag" "$(grep -c 'dev-amd64-' "$base/push-dev/docker.log" || true)" "0"
need "dev commit tag" "$(grep -c -- '--tag ghcr.io/igs170911/anybot:dev-bbbbbbbbbbbb' "$base/push-dev/docker.log" || true)" "1"
need "dev did not move :dev yet" "$(grep -c -- '--tag ghcr.io/igs170911/anybot:dev ' "$base/push-dev/docker.log" || true)" "0"
need "dev did not use the stable short tag" "$(grep -c 'anybot:bbbbbbbbbbbb ' "$base/push-dev/docker.log" || true)" "0"
need "dev has no version tag" "$(grep 'imagetools create' "$base/push-dev/docker.log" | grep -c -- '--tag ' || true)" "1"

say "--- a real push uses the digest docker printed ---"
prepare push-real
: > "$base/push-real-out"
set +e
IMAGE_TAG=0.1.0 COMMIT=aaaaaaaaaaaa IMAGE=ghcr.io/igs170911/anybot \
  CHANNEL=stable IMAGE_DIR="$base/parts" IMAGE_META="$base/real-index.json" \
  GITHUB_OUTPUT="$root/$base/push-real-out" \
  sh deploy/release/push-images.sh >"$base/push-real.out" 2>"$base/push-real.err"
rc=$?
set -e
need_ok "real push" "$rc"
need "real push has no verify tag" "$(grep -c ':verify' "$base/push-real/docker.log" || true)" "0"
need "index built from pushed digests" "$(grep -c '@sha256:pushed' "$base/push-real/docker.log" || true)" "1"
need "real index digest" "$(grep '^digest=' "$base/push-real-out" | cut -d= -f2-)" "sha256:index"
need "real arch tag is commit scoped" "$(grep -c 'amd64-aaaaaaaaaaaa' "$base/push-real/docker.log" || true)" "2"
need "real version tag planned" "$(grep -c -- '--tag ghcr.io/igs170911/anybot:0.1.0' "$base/push-real/docker.log" || true)" "1"

say "--- :dev moves only when asked, onto the dev commit tag ---"
prepare move-dev
set +e
IMAGE=ghcr.io/igs170911/anybot COMMIT=bbbbbbbbbbbb \
  sh deploy/release/move-dev.sh >"$base/move-dev.out" 2>"$base/move-dev.err"
rc=$?
set -e
need_ok "move dev" "$rc"
need "moved onto dev commit tag" "$(grep -c 'anybot:dev-bbbbbbbbbbbb' "$base/move-dev/docker.log" || true)" "1"
need "recheck is the first publish step" "$(
  awk '
    /^  publish:/{job=1}
    job && /^  latest:/{exit}
    job && /^      - name: /{print; exit}
  ' .github/workflows/release.yml
)" "      - name: re-check the dev tip before any write"

if [ "$skip_to_images" -eq 0 ]; then
say "--- spaces are removed before the checksum ---"
src="$base/spaced"
out="$base/page"
rm -rf "$src" "$out"
mkdir -p "$src"
printf 'exe\n' > "$src/AnyBot Dev_0.1.0_x64-setup.exe"
sh deploy/release/sanitize-names.sh "$src" "$out" >"$base/names.out"
need "space removed" "$(find "$out" -name '* *' | wc -l | tr -d ' ')" "0"
need "hyphenated name" "$(find "$out" -name 'AnyBot-Dev_0.1.0_x64-setup.exe' | wc -l | tr -d ' ')" "1"

publish_work() {
  work="$root/$base/work"
  rm -rf "$work"
  mkdir -p "$work/target/release-page"
  printf 'new\n' > "$work/target/release-page/new.txt"
  printf 'commit bbbbbbbbbbbb digest sha256:ccc\n' > "$work/target/release-notes.md"
}

say "--- dev draft failure leaves the visible page ---"
prepare pub-fail upload-fail
publish_work
set +e
here="$(pwd)"
cd "$work"
CHANNEL=dev TAG=dev-build SHA=bbbbbbbbbbbb IMAGE_DIGEST=sha256:ccc \
  sh "$root/deploy/release/publish-release.sh" >"$root/$base/pub.out" 2>"$root/$base/pub.err"
rc=$?
cd "$here"
set -e
need "draft failure refuses" "$rc" "1"
need "draft failure reason" "$(grep -c 'visible page is unchanged' "$base/pub.err" || true)" "1"
need "tag not moved after a failed draft" "$(grep -c 'git tag' "$base/pub-fail/git.log" || true)" "0"
need "visible page not deleted" "$(grep -c 'release delete dev-build ' "$base/pub-fail/gh.log" || true)" "0"
need "no clobber" "$(grep -c clobber "$base/pub-fail/gh.log" || true)" "0"

say "--- dev swap failure does not move the tag or :dev ---"
prepare pub-swap swap-fail
publish_work
set +e
cd "$work"
CHANNEL=dev TAG=dev-build SHA=bbbbbbbbbbbb IMAGE_DIGEST=sha256:ccc \
  sh "$root/deploy/release/publish-release.sh" >"$root/$base/pub-swap.out" 2>"$root/$base/pub-swap.err"
rc=$?
cd "$here"
set -e
need "swap failure refuses" "$rc" "1"
need "swap failure reason" "$(grep -c 'was not published' "$base/pub-swap.err" || true)" "1"
need "old page deleted with its tag" "$(grep -c 'release delete dev-build --yes --cleanup-tag' "$base/pub-swap/gh.log" || true)" "1"
need "swap did not call docker" "$(wc -c < "$base/pub-swap/docker.log" | tr -d ' ')" "0"
need "swap did not push a git tag" "$(grep -c 'git tag' "$base/pub-swap/git.log" || true)" "0"

say "--- dev upload then one edit publishes onto dev-build ---"
prepare pub-ok
publish_work
set +e
cd "$work"
CHANNEL=dev TAG=dev-build SHA=bbbbbbbbbbbb IMAGE_DIGEST=sha256:ccc \
  sh "$root/deploy/release/publish-release.sh" >"$root/$base/pub2.out" 2>"$root/$base/pub2.err"
rc=$?
cd "$here"
set -e
need "dev publish succeeds" "$rc" "0"
need "draft created first" "$(grep -c 'release create dev-build-next-' "$base/pub-ok/gh.log" || true)" "1"
need "old page deleted with its tag" "$(grep -c 'release delete dev-build --yes --cleanup-tag' "$base/pub-ok/gh.log" || true)" "1"
need "one edit renames and publishes" "$(grep 'release edit' "$base/pub-ok/gh.log" | grep -c -- '--tag dev-build' || true)" "1"
need "that edit publishes" "$(grep 'release edit' "$base/pub-ok/gh.log" | grep -c -- '--draft=false' || true)" "1"
need "only one release edit" "$(grep -c 'release edit' "$base/pub-ok/gh.log" || true)" "1"
need "publish did not push a git tag" "$(grep -c 'git tag' "$base/pub-ok/git.log" || true)" "0"
need "publish itself did not move :dev" "$(wc -c < "$base/pub-ok/docker.log" | tr -d ' ')" "0"

fi

if [ -z "${ANYBOT_SELFTEST_INNER:-}" ]; then
  say "--- ANYBOT_DOCKER cannot point past the stub ---"
  trap_bin="$root/$base/trap-docker"
  printf '%s\n' '#!/bin/sh' 'printf "%s\n" "$*" >> "$ANYBOT_DOCKER_LOG"' 'exit 99' > "$trap_bin"
  chmod 755 "$trap_bin"
  : > "$root/$base/trap-docker.log"
  env ANYBOT_DOCKER="$trap_bin" \
    ANYBOT_DOCKER_LOG="$root/$base/trap-docker.log" \
    ANYBOT_SELFTEST_INNER=1 \
    ANYBOT_SELFTEST_BASE="target/release-selftest-inner" \
    sh "$root/deploy/release/release-selftest.sh" >"$base/nested.out" 2>"$base/nested.err" || fail=1
  need "nested selftest passed" "$(grep -c 'release-selftest: pass' "$base/nested.out" || true)" "1"
  need "ANYBOT_DOCKER was not called" "$(wc -c < "$root/$base/trap-docker.log" | tr -d ' ')" "0"
fi

if [ "$fail" -ne 0 ]; then
  say "release-selftest: failed"
  exit 1
fi
say "release-selftest: pass"
