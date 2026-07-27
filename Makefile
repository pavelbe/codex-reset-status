CARGO ?= $(if $(wildcard $(HOME)/.cargo/bin/cargo),$(HOME)/.cargo/bin/cargo,cargo)
# Optional local wrapper. Example: HEAVY_LOCK=~/.claude/bin/heavy-lock.sh
HEAVY_LOCK ?=

.PHONY: check test fmt clippy build package-local clean

define run_heavy
	@if [ -n "$(HEAVY_LOCK)" ] && [ -x "$(HEAVY_LOCK)" ]; then \
		"$(HEAVY_LOCK)" -- $(1); \
	else \
		$(1); \
	fi
endef

test:
	$(call run_heavy,$(CARGO) test --locked)

fmt:
	$(call run_heavy,$(CARGO) fmt --all -- --check)

clippy:
	$(call run_heavy,$(CARGO) clippy --all-targets --locked -- -D warnings)

build:
	$(call run_heavy,$(CARGO) build --release --locked)

check: test fmt clippy build
	bash scripts/check-no-subprocess.sh
	bash scripts/check-public-claims.sh
	$(call run_heavy,node --test npm/codex-reset-status/test/cli.test.mjs)

package-local:
	$(call run_heavy,env CARGO="$(CARGO)" bash scripts/package-release.sh --output-dir dist)

clean:
	$(CARGO) clean
	rm -rf dist
