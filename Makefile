SHELL = /bin/bash

BULMA_VERSION := 0.9.3
FONTAWESOME_VERSION := 6.1.1

PYTHON_PACKAGES := valens tests
FRONTEND_FILES := index.css manifest.json service-worker.js valens-frontend.js valens-frontend_bg.wasm fonts images js

export SQLALCHEMY_WARN_20=1

.PHONY: all

all: check test test_installation

.PHONY: check check_frontend check_backend check_black check_isort check_pylint check_mypy

check: check_frontend check_backend

check_frontend:
	cargo fmt --manifest-path=frontend/Cargo.toml -- --check
	cargo check --manifest-path=frontend/Cargo.toml
	cargo clippy --manifest-path=frontend/Cargo.toml

check_backend: check_black check_isort check_pylint check_mypy

check_black:
	black --check --diff --line-length 100 $(PYTHON_PACKAGES)

check_isort:
	isort --check --diff $(PYTHON_PACKAGES)

check_pylint:
	pylint $(PYTHON_PACKAGES)

check_mypy:
	mypy --pretty $(PYTHON_PACKAGES)

.PHONY: format

format:
	cargo fmt --manifest-path=frontend/Cargo.toml
	black -l 100 $(PYTHON_PACKAGES)
	isort $(PYTHON_PACKAGES)

.PHONY: test test_frontend test_backend test_installation

test: test_frontend test_backend

test_frontend:
	cargo test --manifest-path=frontend/Cargo.toml

test_backend:
	mkdir -p valens/frontend
	touch $(addprefix valens/frontend/,$(FRONTEND_FILES))
	python3 -m pytest -n$(shell nproc) -vv --cov=valens --cov-branch --cov-fail-under=100 --cov-report=term-missing:skip-covered tests

test_installation: dist
	$(eval TMPDIR := $(shell mktemp -d))
	python3 -m venv $(TMPDIR)/venv
	$(TMPDIR)/venv/bin/pip install dist/valens-`python3 -c 'import setuptools_scm; print(setuptools_scm.get_version())'`-py3-none-any.whl
	$(TMPDIR)/venv/bin/valens --help
	rm -rf $(TMPDIR)

.PHONY: update update_css update_fonts

update: update_css update_fonts

update_css: third-party/bulma
	sass --no-source-map sass/bulma.scss:valens/static/css/bulma.css

update_fonts: third-party/fontawesome
	cp third-party/fontawesome/webfonts/fa-solid-900.{woff2,ttf} frontend/assets/fonts/

third-party/bulma:
	wget -qO- https://github.com/jgthms/bulma/releases/download/$(BULMA_VERSION)/bulma-$(BULMA_VERSION).zip | bsdtar -xf- -C third-party
	rm -rf third-party/bulma/css

third-party/fontawesome:
	wget -qO- https://use.fontawesome.com/releases/v$(FONTAWESOME_VERSION)/fontawesome-free-$(FONTAWESOME_VERSION)-web.zip | bsdtar -xf- -C third-party
	rm -rf third-party/fontawesome
	mv third-party/fontawesome-* third-party/fontawesome
	rm -rf third-party/fontawesome/{css,js,less,metadata,sprites,svgs}

.PHONY: dist

dist: valens/frontend $(addprefix valens/frontend/,$(FRONTEND_FILES))
	python3 -m build

valens/frontend:
	mkdir -p valens/frontend

valens/frontend/%: frontend/dist/%
	cp -r $< $@

$(addprefix frontend/dist/,$(FRONTEND_FILES)): third-party/bulma third-party/fontawesome $(shell find frontend/src/ -type f -name '*.rs')
	cd frontend && trunk build --release --filehash false

.PHONY: run run_frontend run_backend

run:
	FLASK_ENV=development FLASK_APP=valens.web VALENS_CONFIG=${PWD}/config.py flask run -h 0.0.0.0

run_frontend:
	PATH=~/.cargo/bin:${PATH} trunk --config frontend/Trunk.toml serve --port 8000

run_backend:
	FLASK_ENV=development FLASK_APP=valens VALENS_CONFIG=${PWD}/config.py flask run -h 0.0.0.0

.PHONY: clean

clean:
	rm -rf valens.egg-info
	rm -rf valens/frontend
	cd frontend && trunk clean && cargo clean
