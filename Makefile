# AnyBotNext · 檢查關卡（spec/05 §1 第 2 步）
# make check 依序跑全部關卡，任何一步失敗就整個失敗。
# Rust 關卡在前（M1），TypeScript 關卡在後（M2），Tauri 的 Rust 部分走原本的 Rust 關卡。

CARGO ?= cargo
PNPM ?= pnpm
NEXTTEST := $(shell command -v cargo-nextest 2>/dev/null)

# spec-trace 基準比對的 ref：本機跟 HEAD 比；CI 跟主分支的分岔點比（CI 設這個變數）。
SPEC_TRACE_REF ?= HEAD

.PHONY: check check-fast fmt clippy test deny machete depcheck spec-trace spec-baseline \
	ironlaws gitleaks ts-install ts-biome ts-typecheck ts-test ts-build \
	api-docs api-lint api-client api-html api-compat

check: fmt clippy test deny machete depcheck spec-trace ironlaws gitleaks \
	api-docs api-lint api-client api-html api-compat \
	ts-install ts-biome ts-typecheck ts-test ts-build

# 邊寫邊跑：最快的關卡＋四條鐵律的檢查（cargo fmt 檢查、clippy、depcheck、iron-laws、biome）。
check-fast: fmt clippy depcheck ironlaws api-docs api-lint ts-biome

fmt:
	$(CARGO) fmt --all --check

# Clippy 只讀「執行時的最近設定檔」，不會合併：逐套件在自己的資料夾裡跑，
# 准許豁免的套件才會讀到自己的 clippy.toml（spec/09 K75）。
clippy:
	@for manifest in $(shell $(CARGO) metadata --format-version 1 --no-deps | jq -r '.packages[].manifest_path'); do \
		(cd "$$(dirname $$manifest)" && $(CARGO) clippy --all-targets -- -D warnings) || exit 1; \
	done

test:
ifdef NEXTTEST
	$(CARGO) nextest run --workspace
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

# 掃金鑰：整個 repo 含 git 歷史（spec/09 K79）。
gitleaks:
	gitleaks detect --source . --no-banner --exit-code 1

# 明確指令：重新產生 spec-trace 基準檔（CI 不跑；只有人要重新產生時跑）。
spec-baseline:
	$(CARGO) run --locked -p spec-trace -- --write-baseline

# TypeScript 關卡（M2 第 3 項，依序）。
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

# API 文件關卡（spec/02 §8）：重新產生跟已提交的檔一比，有差異就擋（文件跟著程式碼走）。
api-docs:
	mkdir -p target/api
	$(CARGO) run --locked -p api-docs -- openapi target/api/openapi.yaml
	$(CARGO) run --locked -p api-docs -- asyncapi target/api/asyncapi.yaml
	diff api/openapi.yaml target/api/openapi.yaml
	diff api/asyncapi.yaml target/api/asyncapi.yaml

api-lint:
	$(PNPM) exec redocly lint --config redocly.yaml api/openapi.yaml
	$(PNPM) exec redocly lint --config redocly-asyncapi.yaml api/asyncapi.yaml

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

# 相容性檢查（spec/02 §8）：跟 git 上已提交的版本比，破壞性變更就擋。
api-compat:
	mkdir -p target/api
	@if git cat-file -e HEAD:api/openapi.yaml 2>/dev/null; then \
		git show HEAD:api/openapi.yaml > target/api/baseline.yaml; \
		$(CARGO) run --locked -p api-docs -- openapi target/api/fresh.yaml; \
		oasdiff breaking target/api/baseline.yaml target/api/fresh.yaml -o ERR; \
	fi
