# AnyBotNext · 檢查關卡（spec/05 §1 第 2 步）
# make check 依序跑全部關卡，任何一步失敗就整個失敗。
# Rust 關卡在前（M1），TypeScript 關卡在後（M2），Tauri 的 Rust 部分走原本的 Rust 關卡。

CARGO ?= cargo
PNPM ?= pnpm
NEXTTEST := $(shell command -v cargo-nextest 2>/dev/null)

# spec-trace 基準比對的 ref：本機跟 HEAD 比；CI 跟主分支的分岔點比（CI 設這個變數）。
SPEC_TRACE_REF ?= HEAD

.PHONY: check check-fast fmt clippy test deny machete depcheck spec-trace spec-baseline \
	ts-install ts-biome ts-typecheck ts-test ts-build

check: fmt clippy test deny machete depcheck spec-trace ts-install ts-biome ts-typecheck ts-test ts-build

# 邊寫邊跑：只跑最快的四個（cargo fmt 檢查、clippy、depcheck、biome）。
check-fast: fmt clippy depcheck ts-biome

fmt:
	$(CARGO) fmt --all --check

clippy:
	$(CARGO) clippy --workspace --all-targets -- -D warnings

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

# 明確指令：重新產生 spec-trace 基準檔（CI 不跑；只有人要重新產生時跑）。
spec-baseline:
	$(CARGO) run --locked -p spec-trace -- --write-baseline

# TypeScript 關卡（M2 第 3 項，依序）。
ts-install:
	$(PNPM) install --frozen-lockfile

ts-biome:
	$(PNPM) exec biome ci .

ts-typecheck:
	$(PNPM) -r --no-bail typecheck

ts-test:
	$(PNPM) -r --no-bail test

ts-build:
	$(PNPM) -r --no-bail build
