SHELL = /bin/bash

BULMA_VERSION := 1.0.4
BULMA_SLIDER_VERSION := 2.0.5
FONTAWESOME_VERSION := 7.2.0

PYTHON_PACKAGES := valens tests tools fabfile.py
ASSETS_DIR := valens/static/assets
GENERATED_DIR := valens/static/generated
GENERATED_FILES := main.css valens-web-app-dioxus.js valens-web-app-dioxus_bg.wasm
PACKAGE_GENERATED_FILES := $(addprefix $(GENERATED_DIR)/,$(GENERATED_FILES))
BUILD_DIR := $(PWD)/build
CONFIG_FILE := $(BUILD_DIR)/config.py
WEBDRIVER_CONFIG := $(BUILD_DIR)/webdriver.json
VERSION := $(lastword $(shell env -u FORCE_COLOR uv run -- hatch version 2>/dev/null))
VERSION_PUBLIC := $(firstword $(subst +, ,$(VERSION)))
WHEEL := dist/valens-$(VERSION)-py3-none-any.whl
# `nproc` is not available on macOS
NPROC := $(shell nproc 2>/dev/null || sysctl -n hw.ncpu)
# The browser used for the end-to-end tests. Only Chromium is selected by its channel, which
# ensures that the browser provided by the development environment is used.
BROWSER ?= chromium
ifeq ($(BROWSER),chromium)
PYTEST_BROWSER := --browser-channel chromium
else
PYTEST_BROWSER := --browser $(BROWSER)
endif
# An optional Playwright device profile, which emulates viewport, user agent and touch support
DEVICE ?=
ifneq ($(DEVICE),)
PYTEST_DEVICE := --device "$(DEVICE)"
endif
_ := $(shell mkdir -p $(BUILD_DIR) && { printf '%s' '$(VERSION)' | cmp -s - $(BUILD_DIR)/version 2>/dev/null || printf '%s' '$(VERSION)' > $(BUILD_DIR)/version; })

export SQLALCHEMY_WARN_20=1

.PHONY: all

all: check test

.PHONY: check check-project check-lockfile check-kacl check-doc check-workflows check-playwright check-rustfmt check-wasm-bindgen check-frontend check-backend check-ruff-format check-ruff check-mypy

check: check-project check-frontend check-backend

check-project: check-lockfile check-kacl check-doc check-workflows check-playwright

check-lockfile:
	uv lock --locked

check-kacl:
	uv run -- kacl-cli verify

check-doc:
	uv run -- python tools/check_doc_links.py

check-workflows:
	actionlint

check-playwright:
	@if [ -n "$$PLAYWRIGHT_BROWSERS_PATH" ]; then \
		browsers=$$(uv run -- playwright install --dry-run $(BROWSER) \
			| sed -n 's/^ *Install location: *//p'); \
		[ -n "$$browsers" ] \
			|| { echo "Error: browsers expected by playwright could not be determined" >&2; exit 1; }; \
		for browser in $$browsers; do \
			[ -e "$$browser" ] \
				|| { echo "Error: $${browser##*/} is not provided by PLAYWRIGHT_BROWSERS_PATH" >&2; exit 1; }; \
		done; \
	fi

# Stable rustfmt ignores the unstable options in `rustfmt.toml` with a warning instead of failing.
check-rustfmt:
	@$${RUSTFMT:-rustfmt} --version | grep -q nightly \
		|| { echo "Error: rustfmt is not the nightly build pinned in rustfmt-toolchain.toml, see doc/DEVELOPMENT.md" >&2; exit 1; }

# The wasm-bindgen version of `dx` can only be determined for a Nix build.
check-wasm-bindgen:
	@dx=$$(readlink -f "$$(command -v dx)" 2>/dev/null); \
	case "$$dx" in /nix/store/*) ;; *) exit 0;; esac; \
	expected=$$(sed -n '/^name = "wasm-bindgen"$$/{n; s/^version = "\(.*\)"$$/\1/p;}' Cargo.lock); \
	[ -n "$$expected" ] \
		|| { echo "Error: wasm-bindgen version required by the project could not be determined" >&2; exit 1; }; \
	actual=$$(nix-store -q --references "$$dx" | sed -n 's|.*-wasm-bindgen-cli-\(.*\)|\1|p' | head -1); \
	[ -n "$$actual" ] \
		|| { echo "Error: wasm-bindgen version used by dx could not be determined" >&2; exit 1; }; \
	[ "$$actual" = "$$expected" ] \
		|| { echo "Error: dx uses wasm-bindgen $$actual, but the project requires $$expected" >&2; exit 1; }

check-frontend: check-rustfmt check-wasm-bindgen
	cargo fmt -- --check
	cargo clippy --all-targets -- --warn clippy::pedantic --deny warnings
	RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --workspace
	dx check -p valens-web-app-dioxus

check-backend: check-ruff-format check-ruff check-mypy

check-ruff-format:
	uv run -- ruff format --check --diff $(PYTHON_PACKAGES)

check-ruff:
	uv run -- ruff check $(PYTHON_PACKAGES)

check-mypy:
	uv run -- mypy --pretty $(PYTHON_PACKAGES)

.PHONY: format

format: check-rustfmt
	cargo fmt
	uv run -- ruff check --fix-only $(PYTHON_PACKAGES) | true
	uv run -- ruff format $(PYTHON_PACKAGES)

.PHONY: test test-frontend test-backend test-installation test-e2e test-venv

test: test-frontend test-backend test-installation test-e2e

# Without an explicit binary, ChromeDriver uses the first Chrome-like browser found in `PATH`, which
# can be a browser with a version incompatible to the driver.
test-frontend:
	cargo llvm-cov nextest --no-fail-fast
	@browser=$$(command -v chromium) \
		|| { echo "Error: chromium could not be found" >&2; exit 1; }; \
	printf '{"goog:chromeOptions": {"binary": "%s"}}\n' "$$browser" > $(WEBDRIVER_CONFIG)
	WASM_BINDGEN_TEST_WEBDRIVER_JSON=$(WEBDRIVER_CONFIG) wasm-pack test --headless --chrome crates/storage

test-backend:
	mkdir -p $(GENERATED_DIR)
	$(foreach f,$(PACKAGE_GENERATED_FILES),test -f $(f) || touch $(f);)
	uv run -- pytest -n$(NPROC) -vv --cov=valens --cov-branch --cov-fail-under=100 --cov-report=term-missing:skip-covered tests/backend
	find $(PACKAGE_GENERATED_FILES) -type f -empty -delete

test-installation: test-venv
	$(BUILD_DIR)/venv/bin/valens --version

test-e2e: check-playwright test-venv
	@grep -qaF "$(VERSION)" $(GENERATED_DIR)/valens-web-app-dioxus_bg.wasm || { echo "Error: $(GENERATED_DIR)/valens-web-app-dioxus_bg.wasm does not contain current version string \"$(VERSION)\"" >&2; exit 1; }
	uv run -- pytest -n$(NPROC) -vv $(PYTEST_BROWSER) $(PYTEST_DEVICE) --reruns 1 --maxfail 3 --tracing retain-on-failure tests/e2e

test-venv: $(BUILD_DIR)/venv/bin/valens

$(BUILD_DIR)/venv:
	python3 -m venv $(BUILD_DIR)/venv

$(BUILD_DIR)/venv/bin/valens: $(BUILD_DIR)/venv $(WHEEL)
	$(BUILD_DIR)/venv/bin/pip install --force-reinstall $(WHEEL)
	test -f $(BUILD_DIR)/venv/bin/valens
	@generated_dir=$$($(BUILD_DIR)/venv/bin/python -c "import pathlib, sysconfig; print(pathlib.Path(sysconfig.get_paths()['purelib']) / 'valens' / 'static' / 'generated')"); \
	for f in $(GENERATED_FILES); do \
		[ -s "$$generated_dir/$$f" ] || { echo "Error: $$f is missing or empty in the installed package" >&2; exit 1; }; \
	done
	# `-c` is the portable spelling of `--no-create`, which BSD `touch` does not support
	touch -c $(BUILD_DIR)/venv/bin/valens

.PHONY: update update-fonts

update: update-fonts third-party/bulma third-party/bulma-slider

update-fonts: third-party/fontawesome
	cp third-party/fontawesome/webfonts/fa-solid-900.woff2 $(ASSETS_DIR)/fonts/

third-party/bulma:
	wget -qO- https://github.com/jgthms/bulma/releases/download/$(BULMA_VERSION)/bulma-$(BULMA_VERSION).zip | bsdtar -xf- -C third-party
	rm -rf third-party/bulma/css

third-party/bulma-slider:
	wget -qO- https://github.com/Wikiki/bulma-slider/archive/refs/tags/v$(BULMA_SLIDER_VERSION).tar.gz | bsdtar -xf- -C third-party
	mv third-party/bulma-slider-$(BULMA_SLIDER_VERSION) third-party/bulma-slider
	rm -rf third-party/bulma-slider/{.*,dist,src/js,test,*.js,*.json,*.png}

third-party/fontawesome:
	wget -qO- https://use.fontawesome.com/releases/v$(FONTAWESOME_VERSION)/fontawesome-free-$(FONTAWESOME_VERSION)-web.zip | bsdtar -xf- -C third-party
	rm -rf third-party/fontawesome
	mv third-party/fontawesome-* third-party/fontawesome
	rm -rf third-party/fontawesome/{css,js,less,metadata,sprites,sprites-full,svgs,svgs-full}

.PHONY: screenshots check-screenshots check-fonts

CREATE_SCREENSHOTS_CMD = uv run -- python tools/create_screenshots.py \
	--valens $(BUILD_DIR)/venv/bin/valens

screenshots: check-playwright check-fonts test-venv
	$(CREATE_SCREENSHOTS_CMD)

check-screenshots: check-playwright check-fonts test-venv
	$(CREATE_SCREENSHOTS_CMD) --check

check-fonts:
	@case "$$FONTCONFIG_FILE" in \
		/nix/store/*) ;; \
		*) echo "Error: fonts are not pinned by FONTCONFIG_FILE, see doc/DEVELOPMENT.md" >&2; exit 1;; \
	esac

.PHONY: check-readme

check-readme: $(WHEEL)
	uv run -- python tools/check_readme_links.py $(WHEEL)

.PHONY: dist

dist: $(WHEEL)

$(WHEEL): $(PACKAGE_GENERATED_FILES)
	uv build

$(PACKAGE_GENERATED_FILES): DX_RELEASE_DIR := target/dx/valens-web-app-dioxus/release/web/public
$(PACKAGE_GENERATED_FILES): third-party/bulma third-party/bulma-slider third-party/fontawesome $(shell find crates/ -type f) $(BUILD_DIR)/version
	mkdir -p $(GENERATED_DIR)
	rm -rf $(GENERATED_DIR)/*
	sass crates/web-app-dioxus/assets/main.scss $(GENERATED_DIR)/main.css
	rm -rf $(DX_RELEASE_DIR)
	VALENS_VERSION=$(VERSION) dx bundle --release --debug-symbols=false --package valens-web-app-dioxus
	# `dx` hashes asset file names per build. Resolving them via `index.html` selects the assets of
	# the current build even if files of older builds are present. `\w` is avoided in the patterns,
	# as it is a GNU extension which is unsupported by the BSD tools on macOS.
	js=$$(grep -o 'assets/valens-web-app-dioxus-dx[[:alnum:]_]*\.js' $(DX_RELEASE_DIR)/index.html | head -1); \
	[ -n "$$js" ] || { echo "Error: JS asset could not be determined" >&2; exit 1; }; \
	wasm=$$(grep -o 'valens-web-app-dioxus_bg-dx[[:alnum:]_]*\.wasm' $(DX_RELEASE_DIR)/$$js | head -1); \
	[ -n "$$wasm" ] || { echo "Error: WASM asset could not be determined" >&2; exit 1; }; \
	sed -e "s#/./assets/#/#" -e "s#-dx[[:alnum:]_]*##" $(DX_RELEASE_DIR)/$$js > $(GENERATED_DIR)/valens-web-app-dioxus.js; \
	cp $(DX_RELEASE_DIR)/assets/$$wasm $(GENERATED_DIR)/valens-web-app-dioxus_bg.wasm
	@refs=$$(grep -o 'valens-web-app-dioxus_bg[[:alnum:]_-]*\.wasm' $(GENERATED_DIR)/valens-web-app-dioxus.js | sort -u); \
	[ "$$refs" = "valens-web-app-dioxus_bg.wasm" ] || { echo "Error: $(GENERATED_DIR)/valens-web-app-dioxus.js references unexpected WASM assets:" >&2; echo "$$refs" >&2; exit 1; }

.PHONY: container container-script

BUILD_CONTAINER_CMD = $(TOOL) build \
	--build-arg WHEEL=$(WHEEL) \
	--build-arg VERSION=$(VERSION) \
	--build-arg REVISION=$(REVISION) \
	--build-arg SOURCE=$(SOURCE) \
	-t $(NAME):$(VERSION_PUBLIC) \
	$(ARGS) \
	.

container: NAME ?= valens
container: TOOL ?= podman
container: $(WHEEL)
	$(BUILD_CONTAINER_CMD)

container-script: NAME ?= valens
container-script: TOOL ?= podman
container-script: BUILD_CONTAINER_SCRIPT := $(BUILD_DIR)/container.sh
container-script:
	echo "#!/bin/sh" > $(BUILD_CONTAINER_SCRIPT)
	echo $(BUILD_CONTAINER_CMD) >> $(BUILD_CONTAINER_SCRIPT)
	chmod +x $(BUILD_CONTAINER_SCRIPT)

.PHONY: run run-frontend run-backend

run:
	tmux new-window $(MAKE) CONFIG_FILE=$(CONFIG_FILE) run-frontend
	tmux new-window $(MAKE) CONFIG_FILE=$(CONFIG_FILE) run-backend

DETECT_HOST := if [ -f /run/.containerenv ] || [ -f /.dockerenv ]; then echo "0.0.0.0"; else echo "127.0.0.1"; fi

run-frontend:
	mkdir -p target/dx/valens-web-app-dioxus/debug/web/public/
	cp -r valens/static/assets/{fonts,images,favicon.ico,manifest.json,sw.js} target/dx/valens-web-app-dioxus/debug/web/public/
	sass --update crates/web-app-dioxus/assets/main.scss target/dx/valens-web-app-dioxus/debug/web/public/main.css
	dx serve --package valens-web-app-dioxus --addr $$($(DETECT_HOST))

run-backend: $(CONFIG_FILE)
	VALENS_CONFIG=$(CONFIG_FILE) uv run -- flask --app valens --debug run -h $$($(DETECT_HOST))

$(CONFIG_FILE): $(BUILD_DIR)
	uv run -- valens config -d build

$(BUILD_DIR):
	mkdir -p $(BUILD_DIR)

.PHONY: clean

clean:
	rm -rf $(BUILD_DIR)
	rm -rf $(GENERATED_DIR)
	rm -rf dist
	rm -rf target
	rm -rf test-results
	rm -rf valens.egg-info
	rm -rf valens/static/generated

.PHONY: version version-public release-notes

version:
	@echo $(VERSION)

version-public:
	@echo $(VERSION_PUBLIC)

release-notes: VERSION_TAG ?= $(shell uv run -- kacl-cli current)
release-notes:
	@uv run -- kacl-cli get $(VERSION_TAG) --no-header

.PHONY: release tag

release: CURRENT_VERSION = $(shell uv run -- kacl-cli current)
release: check-playwright check-fonts test-venv
	@echo "$(RELEASE_VERSION)" | grep -qE '^[0-9]+\.[0-9]+\.[0-9]+$$' \
		|| { echo "Error: RELEASE_VERSION must be a release version, e.g. make release RELEASE_VERSION=1.2.3"; exit 1; }
	@[ "$(RELEASE_VERSION)" != "$(CURRENT_VERSION)" ] \
		&& [ "$$(printf '%s\n%s\n' "$(CURRENT_VERSION)" "$(RELEASE_VERSION)" | sort -V | tail -1)" = "$(RELEASE_VERSION)" ] \
		|| { echo "Error: release version $(RELEASE_VERSION) is not greater than current version $(CURRENT_VERSION)"; exit 1; }
	@git diff --quiet HEAD \
		|| { echo "Error: repository contains uncommitted changes"; exit 1; }
	$(CREATE_SCREENSHOTS_CMD) --check \
		|| { $(CREATE_SCREENSHOTS_CMD) && git commit -m "Update screenshots" doc/screenshots.png; }
	sed -i -E '/^replacement = /s|(treiher/valens/(blob/)?)[^/]+/|\1v$(RELEASE_VERSION)/|' pyproject.toml
	@! grep '^replacement = ' pyproject.toml | grep -qv '/valens/\(blob/\)\?v$(RELEASE_VERSION)/' \
		|| { echo "Error: README links are not pinned to v$(RELEASE_VERSION)"; exit 1; }
	uv run -- kacl-cli release $(RELEASE_VERSION) --modify
	git add CHANGELOG.md pyproject.toml
	git commit -m "Add $(RELEASE_VERSION) to changelog"

# The version is taken from the changelog, as commit IDs and messages change when a PR is merged.
tag: RELEASE_VERSION = $(shell uv run -- kacl-cli current)
tag:
	@echo "$(RELEASE_VERSION)" | grep -qE '^[0-9]+\.[0-9]+\.[0-9]+$$' \
		|| { echo "Error: no release version found in changelog"; exit 1; }
	@[ "$$(git rev-parse --abbrev-ref HEAD)" = "main" ] \
		|| { echo "Error: main branch is not checked out"; exit 1; }
	git tag -a v$(RELEASE_VERSION) -m ""
