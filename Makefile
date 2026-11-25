# AnyBotNext: check gates (spec/05 section 1 step 2)
# make check runs every gate in order; any step failing fails the whole run.
# Rust gates first (M1), TypeScript gates after (M2); the Rust part of Tauri uses the Rust gates.

CARGO ?= cargo
PNPM ?= pnpm
SHELL := /bin/bash
NEXTTEST := $(shell command -v cargo-nextest 2>/dev/null)

# spec-trace baseline compare ref: locally compares against HEAD; CI compares against the merge-base of the main branch (CI sets this variable).
SPEC_TRACE_REF ?= HEAD

.PHONY: check check-fast fmt clippy test deny machete depcheck spec-trace spec-baseline \
	ironlaws gitleaks ts-install ts-biome ts-typecheck ts-test ts-build \
	api-docs api-lint api-client api-html api-compat api-lint-selftest ts-licenses ts-licenses-selftest \
	e2e \
	locale locale-refresh

check: fmt clippy test deny machete depcheck spec-trace ironlaws gitleaks locale \
	api-docs api-lint api-client api-html api-compat api-lint-selftest \
	ts-install ts-licenses ts-biome ts-typecheck ts-test ts-build e2e

# fast loop: the quickest gates plus the four iron laws (cargo fmt check, clippy, depcheck, iron-laws, biome).
check-fast: fmt clippy depcheck ironlaws locale api-docs api-lint api-client api-html api-lint-selftest ts-licenses ts-biome

fmt:
	$(CARGO) fmt --all --check

# Clippy reads only the nearest config at run time and does not merge: each package runs clippy in its own folder,
# only exempt packages pick up their own clippy.toml (spec/09 K75).
clippy:
	@for manifest in $(shell $(CARGO) metadata --format-version 1 --no-deps | jq -r '.packages[].manifest_path'); do \
		(cd "$$(dirname $$manifest)" && $(CARGO) clippy --all-targets -- -D warnings) || exit 1; \
	done

test:
ifdef NEXTTEST
	$(CARGO) nextest run --workspace --test-threads 1
else
	$(CARGO) test --workspace
endif

deny:
	$(CARGO) deny check

machete:
	$(CARGO) machete

depcheck:
	$(CARGO) run --locked -p depcheck

spec-trace:
	$(CARGO) run --locked -p spec-trace -- --against $(SPEC_TRACE_REF)

ironlaws:
	$(CARGO) run --locked -p iron-laws

# locale gates (spec/09 K91): CJK scan of source files plus translation consistency
# plus generated-code freshness (locale-gen check).
locale:
	$(CARGO) run --locked -p locale-gen -- check

# explicit command: regenerate the committed message code tables (CI does not run it).
locale-refresh:
	$(CARGO) run --locked -p locale-gen -- generate

# key scan: the whole repo including git history (spec/09 K79).
gitleaks:
	gitleaks detect --source . --no-banner --exit-code 1

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
	$(PNPM) -r --no-bail test

ts-build:
	$(PNPM) -r --no-bail build

# M4 section B: Playwright scenarios against apps/web, driven by the testenv executable.
e2e: ts-install
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
	@$(PNPM) -r licenses list --prod --json > target/licenses.json
	@$(LICENSE_ALLOW_CMD) > target/allowed-licenses.txt
	@LC_ALL=C comm -23 <(jq -r 'keys[] | split(" OR ")[]' target/licenses.json | LC_ALL=C sort -u) target/allowed-licenses.txt > target/license-reject.txt
	@! test -s target/license-reject.txt || { echo "TS production dependencies have a disallowed license:"; cat target/license-reject.txt; exit 1; }
	@echo "ts-licenses OK"

ts-licenses-selftest:
	@$(LICENSE_ALLOW_CMD) > target/allowed-licenses.txt
	@LC_ALL=C comm -23 <(jq -r 'keys[] | split(" OR ")[]' tests/fixtures/licenses/forbidden.json | LC_ALL=C sort -u) target/allowed-licenses.txt > target/license-selftest-reject.txt
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

# compatibility check (spec/02 section 8): compare against the committed version in git; breaking changes block.
api-compat:
	mkdir -p target/api
	@if git cat-file -e $(SPEC_TRACE_REF):api/openapi.yaml 2>/dev/null; then \
		git show $(SPEC_TRACE_REF):api/openapi.yaml > target/api/baseline.yaml; \
		$(CARGO) run --locked -p api-docs -- openapi target/api/fresh.yaml; \
		oasdiff breaking target/api/baseline.yaml target/api/fresh.yaml -o ERR; \
	fi
