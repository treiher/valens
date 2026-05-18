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
VERSION := $(shell uv run -- hatch version 2>/dev/null)
VERSION_PUBLIC := $(firstword $(subst +, ,$(VERSION)))
WHEEL := dist/valens-$(VERSION)-py3-none-any.whl
_ := $(shell mkdir -p $(BUILD_DIR) && { printf '%s' '$(VERSION)' | cmp -s - $(BUILD_DIR)/version 2>/dev/null || printf '%s' '$(VERSION)' > $(BUILD_DIR)/version; })

export SQLALCHEMY_WARN_20=1

.PHONY: all

all: check test

.PHONY: check check-project check-lockfile check-kacl check-frontend check-backend check-black check-ruff check-mypy

check: check-project check-frontend check-backend

check-project: check-lockfile check-kacl

check-lockfile:
	uv lock --locked

check-kacl:
	uv run -- kacl-cli verify

check-frontend:
	cargo fmt -- --check
	cargo clippy --all-targets -- --warn clippy::pedantic --deny warnings
	dx check -p valens-web-app-dioxus

check-backend: check-black check-ruff check-mypy

check-black:
	uv run -- black --check --diff $(PYTHON_PACKAGES)

check-ruff:
	uv run -- ruff check $(PYTHON_PACKAGES)

check-mypy:
	uv run -- mypy --pretty $(PYTHON_PACKAGES)

.PHONY: format

format:
	cargo fmt
	uv run -- ruff check --fix-only $(PYTHON_PACKAGES) | true
	uv run -- black $(PYTHON_PACKAGES)

.PHONY: test test-frontend test-backend test-installation test-e2e test-venv

test: test-frontend test-backend test-installation test-e2e

test-frontend:
	cargo llvm-cov nextest --no-fail-fast
	wasm-pack test --headless --chrome crates/storage

test-backend:
	mkdir -p $(GENERATED_DIR)
	$(foreach f,$(PACKAGE_GENERATED_FILES),test -f $(f) || touch $(f);)
	uv run -- pytest -n$(shell nproc) -vv --cov=valens --cov-branch --cov-fail-under=100 --cov-report=term-missing:skip-covered tests/backend
	find $(PACKAGE_GENERATED_FILES) -type f -empty -delete

test-installation: test-venv
	$(BUILD_DIR)/venv/bin/valens --version

test-e2e: test-venv
	@grep -qaF "$(VERSION)" $(GENERATED_DIR)/valens-web-app-dioxus_bg.wasm || { echo "ERROR: $(GENERATED_DIR)/valens-web-app-dioxus_bg.wasm does not contain current version string \"$(VERSION)\""; exit 1; }
	uv run -- pytest -n$(shell nproc) -vv --browser-channel chromium --reruns 1 --maxfail 3 --tracing retain-on-failure tests/e2e

test-venv: $(BUILD_DIR)/venv/bin/valens

$(BUILD_DIR)/venv:
	python3 -m venv $(BUILD_DIR)/venv

$(BUILD_DIR)/venv/bin/valens: $(BUILD_DIR)/venv $(WHEEL)
	$(BUILD_DIR)/venv/bin/pip install --force-reinstall $(WHEEL)
	test -f $(BUILD_DIR)/venv/bin/valens
	touch --no-create $(BUILD_DIR)/venv/bin/valens

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

.PHONY: screenshots

screenshots: test-venv
	tools/create_screenshots.py

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
	sed -e "s#/./assets/#/#" -e "s#-dx\w*##" $(DX_RELEASE_DIR)/assets/valens-web-app-dioxus-dx*.js > $(GENERATED_DIR)/valens-web-app-dioxus.js
	cp $(DX_RELEASE_DIR)/assets/valens-web-app-dioxus_bg-dx*.wasm $(GENERATED_DIR)/valens-web-app-dioxus_bg.wasm

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

.PHONY: version version-public

version:
	@echo $(VERSION)

version-public:
	@echo $(VERSION_PUBLIC)
