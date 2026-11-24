# AnyBotNext · 檢查關卡（spec/05 §1 第 2 步）
# make check 依序跑全部關卡，任何一步失敗就整個失敗。

CARGO ?= cargo
NEXTTEST := $(shell command -v cargo-nextest 2>/dev/null)

.PHONY: check fmt clippy test deny machete depcheck spec-trace spec-baseline

check: fmt clippy test deny machete depcheck spec-trace

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
	$(CARGO) run --locked -p spec-trace

# 明確指令：重新產生 spec-trace 基準檔（CI 不跑；只有人要重新產生時跑）。
spec-baseline:
	$(CARGO) run --locked -p spec-trace -- --write-baseline
