SHELL = /bin/bash

BULMA_VERSION := 1.0.0
BULMA_SLIDER_VERSION := 2.0.5
FONTAWESOME_VERSION := 6.1.1

PYTHON_PACKAGES := valens tests tools fabfile.py
FRONTEND_CRATE := crates/web-app-seed
FRONTEND_FILES := index.css manifest.json service-worker.js valens-web-app-seed.js valens-web-app-seed_bg.wasm fonts images js
PACKAGE_FRONTEND_FILES := valens/frontend $(addprefix valens/frontend/,$(FRONTEND_FILES))
BUILD_DIR := $(PWD)/build
CONFIG_FILE := $(BUILD_DIR)/config.py
VERSION ?= $(shell uv run -- hatch version)
WHEEL ?= dist/valens-$(VERSION)-py3-none-any.whl

export SQLALCHEMY_WARN_20=1

.PHONY: all

all: check test

.PHONY: check check_general check_lockfile check_kacl check_frontend check_backend check_black check_ruff check_mypy

check: check_frontend check_backend

check_general: check_lockfile check_kacl

check_lockfile:
	uv lock --locked

check_kacl:
	uv run -- kacl-cli verify

check_frontend:
	cargo fmt -- --check
	cargo check
	cargo clippy -- --warn clippy::pedantic --deny warnings

check_backend: check_lockfile check_black check_ruff check_mypy

check_black:
	uv run -- black --check --diff $(PYTHON_PACKAGES)

check_ruff:
	uv run -- ruff check $(PYTHON_PACKAGES)

check_mypy:
	uv run -- mypy --pretty $(PYTHON_PACKAGES)

.PHONY: format

format:
	cargo fmt
	uv run -- ruff check --fix-only $(PYTHON_PACKAGES) | true
	uv run -- black $(PYTHON_PACKAGES)

.PHONY: test test_frontend test_backend test_e2e

test: test_frontend test_backend test_installation test_e2e

test_frontend:
	cargo llvm-cov nextest --no-fail-fast

test_backend:
	mkdir -p valens/frontend
	touch $(addprefix valens/frontend/,$(FRONTEND_FILES))
	uv run -- pytest -n$(shell nproc) -vv --cov=valens --cov-branch --cov-fail-under=100 --cov-report=term-missing:skip-covered tests/backend

test_installation: $(BUILD_DIR)/venv/bin/valens
	$(BUILD_DIR)/venv/bin/valens --version

test_e2e: $(BUILD_DIR)/venv/bin/valens
	uv run -- pytest -n$(shell nproc) -vv --driver chrome --headless tests/e2e

$(BUILD_DIR)/venv:
	python3 -m venv $(BUILD_DIR)/venv

$(BUILD_DIR)/venv/bin/valens: $(BUILD_DIR)/venv $(WHEEL)
	$(BUILD_DIR)/venv/bin/pip install --force-reinstall $(WHEEL)
	test -f $(BUILD_DIR)/venv/bin/valens
	touch --no-create $(BUILD_DIR)/venv/bin/valens

.PHONY: update update_css update_fonts

update: update_css update_fonts

update_css: third-party/bulma third-party/bulma-slider

update_fonts: third-party/fontawesome
	cp third-party/fontawesome/webfonts/fa-solid-900.{woff2,ttf} $(FRONTEND_CRATE)/assets/fonts/

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
	rm -rf third-party/fontawesome/{css,js,less,metadata,sprites,svgs}

.PHONY: screenshots

screenshots: $(PACKAGE_FRONTEND_FILES)
	tools/create_screenshots.py

.PHONY: dist

dist: $(WHEEL)

$(WHEEL): $(PACKAGE_FRONTEND_FILES)
	uv build

valens/frontend:
	mkdir -p valens/frontend

valens/frontend/%: $(FRONTEND_CRATE)/dist/%
	rm -rf $@
	cp -r $< $@

$(addprefix $(FRONTEND_CRATE)/dist/,$(FRONTEND_FILES)): third-party/bulma third-party/fontawesome $(shell find $(FRONTEND_CRATE)/{assets,src}/ -type f)
	cd $(FRONTEND_CRATE) && trunk build --release --filehash false

.PHONY: run run_frontend run_backend

run:
	tmux new-window $(MAKE) CONFIG_FILE=$(CONFIG_FILE) run_frontend
	tmux new-window $(MAKE) CONFIG_FILE=$(CONFIG_FILE) run_backend

run_frontend:
	PATH=~/.cargo/bin:${PATH} trunk --config $(FRONTEND_CRATE)/Trunk.toml serve --port 8000

run_backend: $(CONFIG_FILE)
	VALENS_CONFIG=$(CONFIG_FILE) uv run -- flask --app valens --debug run -h 0.0.0.0

$(CONFIG_FILE): $(BUILD_DIR)
	uv run -- valens config -d build

$(BUILD_DIR):
	mkdir -p $(BUILD_DIR)

.PHONY: clean clean_all

clean:
	rm -rf $(BUILD_DIR)
	rm -rf valens.egg-info
	rm -rf valens/frontend
	cargo clean
	cd $(FRONTEND_CRATE) && trunk clean
