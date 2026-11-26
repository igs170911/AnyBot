# AnyBotNext: check gates (spec/05 section 1 step 2)
# make check runs every gate in order; any step failing fails the whole run.
# Rust gates first (M1), TypeScript gates after (M2); the Rust part of Tauri uses the Rust gates.

CARGO ?= cargo
PNPM ?= pnpm
SHELL := /bin/bash -o pipefail
NEXTTEST := $(shell command -v cargo-nextest 2>/dev/null)
OASDIFF := $(shell command -v oasdiff 2>/dev/null)

# sqlx compile-time queries build from the committed .sqlx metadata; the live database url reaches sqlx-cli only through the sqlx-prepare target.
export SQLX_OFFLINE := true

# cross-worktree test lock (docs/testing.md, parallel work): tools/testlock.sh takes the lock
# (the lock file lives in the git common directory so every worktree of this repo shares it;
# macOS uses lockf, Linux uses flock, neither present (CI runs one checkout at a time) means
# no lock). The script keeps every exit status; `;` sequences in recipes are blocked by iron-laws.
define locked
	@bash tools/testlock.sh $1
endef

# spec-trace baseline compare ref: locally compares against HEAD; CI compares against the commit pushed to before the push on dev and main, otherwise the merge base of origin/dev (K111; CI sets this variable).
SPEC_TRACE_REF ?= HEAD

.PHONY: check check-fast fmt clippy test test-fast test-basic live-model-check deny machete depcheck spec-trace spec-baseline sqlx-prepare sqlx-check \
	ironlaws gitleaks versions actionlint ts-install ts-biome ts-typecheck ts-test ts-build \
	api-docs api-lint api-client api-html api-compat api-compat-selftest api-lint-selftest ts-licenses ts-licenses-selftest \
	e2e \
	_test-body _test-fast-body _test-basic-body _e2e-body gate-selftest \
	locale locale-refresh release-selftest

check: fmt clippy gate-selftest test deny machete depcheck sqlx-check spec-trace ironlaws gitleaks versions actionlint locale release-selftest \
	api-docs api-lint api-client api-html api-compat api-compat-selftest api-lint-selftest \
	ts-install ts-licenses ts-biome ts-typecheck ts-test ts-build e2e

# fast loop: the quickest gates plus the four iron laws (cargo fmt check, clippy, depcheck, iron-laws, biome).
check-fast: fmt clippy gate-selftest test-fast depcheck sqlx-check ironlaws versions locale api-docs api-lint api-client api-html api-compat api-compat-selftest api-lint-selftest ts-licenses ts-biome

fmt:
	$(CARGO) fmt --all --check

# Clippy reads only the nearest config at run time and does not merge: each package runs clippy in its own folder,
# only exempt packages pick up their own clippy.toml (spec/09 K75).
clippy:
	@mkdir -p target
	@$(CARGO) metadata --format-version 1 --no-deps > target/cargo-metadata.json
	@jq -r '.packages[].manifest_path' target/cargo-metadata.json > target/clippy-manifests.txt
	@test -s target/clippy-manifests.txt || { echo "cargo metadata listed no package manifests"; exit 1; }
	@while read -r manifest; do (cd "$$(dirname "$$manifest")" && $(CARGO) clippy --all-targets -- -D warnings) || exit 1; done < target/clippy-manifests.txt

test:
	$(call locked,$(MAKE) --no-print-directory _test-body)

# the gate (tools/test-gate.sh, shared with gate-selftest): nextest must exit 0 and the log
# must not contain the unknown-keys warning; both halves block.
_test-body:
ifdef NEXTTEST
	@mkdir -p target
	bash tools/test-gate.sh run target/nextest.log $(CARGO) nextest run --workspace --test-threads 1
else
	$(CARGO) test --workspace
endif

# selftest for the gate above: a failing command and a clean-exit command carrying the
# forbidden phrase must both be blocked; a clean command must pass (spec/05 §2, HOTFIX).
gate-selftest:
	@mkdir -p target
	bash tools/test-gate.sh expect-fail target/gate-selftest.log bash -c 'exit 3'
	bash tools/test-gate.sh expect-fail target/gate-selftest.log bash -c 'echo "ignoring unknown configuration keys" >&2; exit 0'
	bash tools/test-gate.sh expect-pass target/gate-selftest.log bash -c 'echo clean gate output'

# Windows and macOS CI (K111): compile is a separate workflow step. This target runs the
# Rust and TypeScript tests that do not need PostgreSQL or a browser.
test-basic:
	$(call locked,$(MAKE) --no-print-directory _test-basic-body)

_test-basic-body:
	@mkdir -p target
ifdef NEXTTEST
	bash tools/test-gate.sh run target/nextest-basic.log $(CARGO) nextest run --workspace --profile quick
else
	$(CARGO) test --workspace
endif
	$(PNPM) -r --no-bail test | tee target/pnpm-test.log

# check-fast Rust tests: everything except the postgres group (spec/09 K90 real DB, slow) and e2e (separate target).
test-fast:
	$(call locked,$(MAKE) --no-print-directory _test-fast-body)

_test-fast-body:
ifdef NEXTTEST
	$(CARGO) nextest run --workspace --profile quick
else
	$(CARGO) test --workspace
endif

# M4 Section A addendum: compare request formats with the real local model. Not part of check, check-fast, or CI.
live-model-check:
	$(CARGO) run --locked -p live-model

deny:
	$(CARGO) deny check

machete:
	$(CARGO) machete

depcheck:
	$(CARGO) build --locked -p depcheck --target-dir target/depcheck-build
	./target/depcheck-build/debug/depcheck

sqlx-prepare:
	tools/sqlx_prepare.sh "$(CARGO)" "$(PNPM)" prepare

sqlx-check:
	tools/sqlx_prepare.sh "$(CARGO)" "$(PNPM)" check

spec-trace:
	$(CARGO) run --locked -p spec-trace -- --against $(SPEC_TRACE_REF)

ironlaws:
	$(CARGO) run --locked -p iron-laws -- $(SPEC_TRACE_REF)

versions:
	$(CARGO) run --locked -p anybot-versions

# locale gates (spec/09 K91): CJK scan of source files plus translation consistency
# plus generated-code freshness (locale-gen check).
locale:
	$(CARGO) run --locked -p locale-gen -- check

release-selftest:
	sh deploy/release/release-selftest.sh

# explicit command: regenerate the committed message code tables (CI does not run it).
locale-refresh:
	$(CARGO) run --locked -p locale-gen -- generate

# key scan: the whole repo including git history (spec/09 K79).
gitleaks:
	gitleaks detect --source . --no-banner --exit-code 1

# Workflow syntax (spec/09 K110). actionlint 1.7.12, sha256 from the release checksums file.
actionlint:
	@mkdir -p target/tools
	@test -x target/tools/actionlint || bash tools/install-actionlint.sh target/tools/actionlint
	target/tools/actionlint -color .github/workflows/*.yml
	bash tools/actionlint-selftest.sh target/tools/actionlint

# explicit command: regenerate the spec-trace baseline (CI does not run it; only when a person regenerates).
spec-baseline:
	$(CARGO) run --locked -p spec-trace -- --write-baseline

# TypeScript gates (M2 item 3, in order).
ts-install:
	$(PNPM) install --frozen-lockfile

ts-biome:
	$(PNPM) exec biome ci . --error-on-warnings

ts-typecheck:
	$(PNPM) -r --no-bail typecheck

ts-test:
	@mkdir -p target
	$(PNPM) -r --no-bail test | tee target/pnpm-test.log

ts-build:
	$(PNPM) -r --no-bail build

# M4 section B: Playwright scenarios against apps/web, driven by the testenv executable.
e2e:
	$(call locked,$(MAKE) --no-print-directory _e2e-body)

_e2e-body: ts-install
	$(CARGO) build -p testenv
	$(PNPM) --filter=@anybot/e2e exec playwright test

# API document gates (spec/02 section 8): regenerate and diff against the committed files; any difference blocks (docs follow the code).
api-docs:
	mkdir -p target/api
	$(CARGO) run --locked -p api-docs -- openapi target/api/openapi.yaml
	$(CARGO) run --locked -p api-docs -- asyncapi target/api/asyncapi.yaml
	diff api/openapi.yaml target/api/openapi.yaml
	diff api/asyncapi.yaml target/api/asyncapi.yaml

api-lint:
	$(PNPM) exec redocly lint --config redocly.yaml api/openapi.yaml
	$(PNPM) exec redocly lint --config redocly-asyncapi.yaml api/asyncapi.yaml

LICENSE_ALLOW_CMD = sed -n "/^\[licenses\]/,/^\[advisories\]/p" deny.toml | sed -n "/^allow = \[/,/^\]/p" | tr -d " \"[]," | grep -v "^allow=$$" | sed "/^$$/d" | LC_ALL=C sort -u

ts-licenses:
	@mkdir -p target
	@$(PNPM) -r licenses list --prod --json > target/licenses.json
	@$(LICENSE_ALLOW_CMD) > target/allowed-licenses.txt
	@jq -r 'keys[] | split(" OR ")[]' target/licenses.json | LC_ALL=C sort -u > target/license-keys.txt
	@LC_ALL=C comm -23 target/license-keys.txt target/allowed-licenses.txt > target/license-reject.txt
	@! test -s target/license-reject.txt || { echo "TS production dependencies have a disallowed license; see target/license-reject.txt"; exit 1; }
	@echo "ts-licenses OK"

ts-licenses-selftest:
	@mkdir -p target
	@$(LICENSE_ALLOW_CMD) > target/allowed-licenses.txt
	@jq -r 'keys[] | split(" OR ")[]' tests/fixtures/licenses/forbidden.json | LC_ALL=C sort -u > target/license-selftest-keys.txt
	@LC_ALL=C comm -23 target/license-selftest-keys.txt target/allowed-licenses.txt > target/license-selftest-reject.txt
	@test -s target/license-selftest-reject.txt || { echo "selftest did not block GPL-3.0"; exit 1; }
	@grep -F GPL-3.0 target/license-selftest-reject.txt
	@echo "ts-licenses-selftest OK"

# selftest: every bad file under tests/fixtures/api-lint must be blocked by its named rule and every good file must pass (spec/09 K86).
api-lint-selftest: ts-install
	$(PNPM) exec redocly lint --config redocly.yaml tests/fixtures/api-lint/base.yaml
	@for pair in "no-5xx.yaml rule/operation-has-5xx" "no-example.yaml rule/media-content-example-required" "no-idempotency.yaml anybot/idempotencyKeyRequired" "no-problem-json.yaml rule/problem-json-in-4xx-5xx" "no-tags.yaml rule/operation-tags-not-empty"; do \
		file=$${pair%% *}; rule=$${pair##* }; \
		out=$$($(PNPM) exec redocly lint --config redocly.yaml tests/fixtures/api-lint/$$file 2>&1); rc=$$?; \
		if [ $$rc -eq 0 ]; then echo "selftest: $$file should have been blocked"; exit 1; fi; \
		if ! echo "$$out" | grep -F "$$rule" > /dev/null; then echo "selftest: $$file did not hit $$rule"; exit 1; fi; \
	done
	@echo "api-lint-selftest OK"

api-client:
	mkdir -p target/api
	$(PNPM) exec openapi-typescript api/openapi.yaml -o target/api/schema.ts
	diff web/api-client/src/generated/schema.ts target/api/schema.ts

api-html:
	mkdir -p target/api/docs
	$(PNPM) exec redocly build-docs api/openapi.yaml -o target/api/docs/index.html
	diff api/docs/index.html target/api/docs/index.html
	$(PNPM) exec redocly build-docs api/asyncapi.yaml -o target/api/docs/asyncapi.html
	diff api/docs/asyncapi.html target/api/docs/asyncapi.html

# compatibility check (spec/02 section 8, spec/09 K118 6): compare the committed baselines
# with the comparison ref; breaking changes block; a regenerated baseline must carry a dated
# record of the destructive change and its reason in api/breaking-changes.log.
api-compat:
	mkdir -p target/api
ifdef OASDIFF
	$(CARGO) run --locked -p api-docs -- openapi target/api/fresh.yaml
	$(CARGO) run --locked -p api-docs -- asyncapi target/api/fresh-asyncapi.yaml
	@oasdiff breaking target/api/fresh.yaml api/openapi.yaml --format markdown || true
	@oasdiff breaking target/api/fresh-asyncapi.yaml api/asyncapi.yaml --format markdown || true
endif
	$(CARGO) run --locked -p api-compat -- --against $(SPEC_TRACE_REF) --repo .

# selftest of the gate above: fixtures cover breaking changes, regenerated baselines
# without a record, and a record that matches (spec/09 K86).
api-compat-selftest:
	@mkdir -p target
	@bash tests/api-compat-selftest/selftest.sh
