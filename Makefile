# Dialectic's everyday commands. `make help` lists them.

DIALECTIC := $(CURDIR)/target/release/dialectic
# Rune's headless flags (--snap, --scene) are in debug builds; release builds leave them out.
DIALECTIC_DEBUG := $(CURDIR)/target/debug/dialectic
DEMO_DIR := $(CURDIR)/target/demo
SCREENSHOT_DIR := $(CURDIR)/target/screenshots
# Screenshots use their own saved-projects store, so your real list stays out of them. --chrome paints the
# traffic lights in and reports any element under them.
SCREENSHOT_DATA_DIR := $(CURDIR)/target/screenshot-data
REPO ?=

.PHONY: help run build test check demo screenshots install clean

help: ## List the targets
	@grep -E '^[a-z]+:.*## ' $(MAKEFILE_LIST) | awk -F':.*## ' '{printf "  make %-12s %s\n", $$1, $$2}'

run: build ## Launch the app (REPO=path opens a repository directly)
	$(DIALECTIC) $(REPO)

build: ## Build the release binary
	cargo build --release

test: ## Run every test (core, CLI and headless GUI)
	cargo test

check: ## Format check, clippy and tests
	cargo fmt --check
	cargo clippy --all-targets -- -D warnings
	cargo test

demo: build ## Make a disposable two-developer repo with two pull requests, then open it
	scripts/create-demo-repository.sh $(DEMO_DIR) $(DIALECTIC)
	$(DIALECTIC) $(DEMO_DIR)

screenshots: ## Render every screen headless into target/screenshots
	cargo build
	scripts/create-demo-repository.sh $(DEMO_DIR) $(DIALECTIC_DEBUG)
	@mkdir -p $(SCREENSHOT_DIR)
	DIALECTIC_DATA_DIR=$(SCREENSHOT_DATA_DIR) $(DIALECTIC_DEBUG) --chrome --snap $(SCREENSHOT_DIR)/welcome.png
	DIALECTIC_DATA_DIR=$(SCREENSHOT_DATA_DIR) $(DIALECTIC_DEBUG) --scene settings --chrome --snap $(SCREENSHOT_DIR)/settings.png >/dev/null
	@for scene in pull-request-list new-pull-request new-pull-request-preview pull-request-overview \
		pull-request-commits pull-request-files pull-request-files-split pull-request-edit pull-request-rebase; do \
		DIALECTIC_DATA_DIR=$(SCREENSHOT_DATA_DIR) $(DIALECTIC_DEBUG) $(DEMO_DIR) --pull-request 1 --scene $$scene --chrome \
			--snap $(SCREENSHOT_DIR)/$$scene.png >/dev/null || exit 1; \
	done
	@# Pull request 2's branches conflict, so its merge is blocked.
	@for scene in pull-request-conflicts pull-request-conflicts-files changes; do \
		DIALECTIC_DATA_DIR=$(SCREENSHOT_DATA_DIR) $(DIALECTIC_DEBUG) $(DEMO_DIR) --pull-request 2 --scene $$scene --chrome \
			--snap $(SCREENSHOT_DIR)/$$scene.png >/dev/null || exit 1; \
	done
	@echo "Screenshots in $(SCREENSHOT_DIR)"

install: ## Put dialectic on your PATH (cargo install), for you and your agents
	cargo install --locked --path .

clean: ## Remove build output, the demo repo and screenshots
	cargo clean
